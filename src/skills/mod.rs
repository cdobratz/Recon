pub mod skill;

use crate::config::TraceLampConfig;
use crate::parsers::Language;
use crate::scanner::suppress;
use crate::scanner::walk;
use crate::scanner::Finding;
use anyhow::Result;
use regex::Regex;
use skill::SkillDefinition;
use std::collections::HashMap;
use std::path::Path;

/// Add a skill from a TOML file
pub fn add_skill(path: &str, config: &TraceLampConfig) -> Result<()> {
    let skill_path = Path::new(path);
    if !skill_path.exists() {
        anyhow::bail!("Skill file not found: {}", path);
    }

    let contents = std::fs::read_to_string(skill_path)?;
    let skill: SkillDefinition = toml::from_str(&contents)?;

    // Copy to skills directory
    let skills_dir = Path::new(&config.skills.directory);
    std::fs::create_dir_all(skills_dir)?;

    let dest = skills_dir.join(format!("{}.toml", skill.name));
    std::fs::copy(skill_path, &dest)?;

    println!("Added skill: {} -> {}", skill.name, dest.display());
    Ok(())
}

/// List all registered skills
pub fn list_skills(config: &TraceLampConfig) -> Result<()> {
    let skills_dir = Path::new(&config.skills.directory);
    if !skills_dir.exists() {
        println!("No skills directory found. Run 'tracelamp skill add <path>' to add skills.");
        return Ok(());
    }

    let skills = load_skills(config)?;
    if skills.is_empty() {
        println!("No skills registered.");
    } else {
        for skill in &skills {
            println!("  {} - {}", skill.name, skill.description);
        }
        println!("\n{} skill(s) registered.", skills.len());
    }
    Ok(())
}

/// Remove a skill by name
pub fn remove_skill(name: &str, config: &TraceLampConfig) -> Result<()> {
    let skills_dir = Path::new(&config.skills.directory);
    let skill_file = skills_dir.join(format!("{}.toml", name));

    if skill_file.exists() {
        std::fs::remove_file(&skill_file)?;
        println!("Removed skill: {}", name);
    } else {
        println!("Skill not found: {}", name);
    }
    Ok(())
}

/// Load all skills from the skills directory
pub fn load_skills(config: &TraceLampConfig) -> Result<Vec<SkillDefinition>> {
    let skills_dir = Path::new(&config.skills.directory);
    let mut skills = Vec::new();

    if !skills_dir.exists() {
        return Ok(skills);
    }

    for entry in std::fs::read_dir(skills_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let contents = std::fs::read_to_string(&path)?;
            let skill: SkillDefinition = toml::from_str(&contents)?;
            skills.push(skill);
        }
    }

    Ok(skills)
}

/// Run one skill's search patterns. This only reads files.
pub fn run_skill(name: &str, target: &str, config: &TraceLampConfig) -> Result<Vec<Finding>> {
    let findings = collect_skill(name, target, config)?;
    println!("{} finding(s) from skill {}.", findings.len(), name);
    for finding in &findings {
        println!(
            "  {}:{} {} {}",
            finding.file, finding.line, finding.severity, finding.title
        );
    }
    Ok(findings)
}

/// Apply a named skill and return its findings. Callers print them.
pub fn collect_skill(name: &str, target: &str, config: &TraceLampConfig) -> Result<Vec<Finding>> {
    let skill = load_one(name, config)?;
    let patterns = compile_patterns(&skill)?;
    if patterns.is_empty() {
        return Ok(Vec::new());
    }
    let root = Path::new(target);
    let ignore: Vec<&str> = config
        .scan
        .ignore_patterns
        .iter()
        .map(|s| s.as_str())
        .collect();
    let mut findings = Vec::new();
    for path in walk::list_files(root, &ignore)? {
        if walk::is_binary_extension(&path) {
            continue;
        }
        if !matches_target(&path, &skill.target_patterns) {
            continue;
        }
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > config.scan.max_file_size {
                continue;
            }
        }
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        if walk::looks_binary(&bytes) {
            continue;
        }
        let source = match String::from_utf8(bytes) {
            Ok(source) => source,
            Err(_) => continue,
        };
        let mut file_findings = Vec::new();
        for (regex, pattern) in &patterns {
            for (index, line) in source.lines().enumerate() {
                if regex.is_match(line) {
                    file_findings.push(skill_finding(&skill, pattern, &path, index + 1, line));
                }
            }
        }
        findings.extend(suppress::apply(file_findings, &source));
    }
    Ok(findings)
}

fn load_one(name: &str, config: &TraceLampConfig) -> Result<SkillDefinition> {
    let path = Path::new(&config.skills.directory).join(format!("{name}.toml"));
    if !path.exists() {
        anyhow::bail!(
            "Skill not found: {name}. Run 'tracelamp skill list' to see registered skills."
        );
    }
    let contents = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&contents)?)
}

fn compile_patterns(skill: &SkillDefinition) -> Result<Vec<(Regex, skill::SearchPattern)>> {
    let mut compiled = Vec::new();
    for pattern in &skill.search_patterns {
        let regex = Regex::new(&pattern.pattern).map_err(|err| {
            anyhow::anyhow!(
                "skill {}: invalid pattern {}: {err}",
                skill.name,
                pattern.pattern
            )
        })?;
        compiled.push((regex, pattern.clone()));
    }
    Ok(compiled)
}

fn matches_target(path: &Path, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return true;
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    patterns.iter().any(|pattern| {
        if let Some(ext) = pattern.strip_prefix("*.") {
            path.extension().and_then(|ext_os| ext_os.to_str()) == Some(ext)
        } else {
            name.as_ref() == pattern.as_str() || path.ends_with(pattern)
        }
    })
}

fn skill_finding(
    skill: &SkillDefinition,
    pattern: &skill::SearchPattern,
    path: &Path,
    line: usize,
    snippet: &str,
) -> Finding {
    let mut metadata = HashMap::new();
    metadata.insert("rule_id".into(), format!("skill/{}", skill.name));
    metadata.insert("skill".into(), skill.name.clone());
    Finding {
        category: skill
            .category
            .clone()
            .unwrap_or_else(|| "skill".to_string()),
        severity: pattern.severity.clone(),
        file: path.display().to_string(),
        line,
        title: format!("{}: {}", skill.name, pattern.description),
        description: pattern.description.clone(),
        snippet: snippet.trim().to_string(),
        language: Language::from_path(path)
            .map(|language| language.name().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_matches_a_line_and_skips_default_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let skills = dir.path().join("skills");
        std::fs::create_dir_all(&skills).unwrap();
        std::fs::write(
            skills.join("todo.toml"),
            r#"
name = "todo"
description = "flag TODO comments"
target_patterns = ["*.py"]
severity = "info"

[[search_patterns]]
pattern = "TODO"
description = "leftover TODO"
severity = "low"
"#,
        )
        .unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join("node_modules")).unwrap();
        std::fs::write(repo.join("app.py"), "print(1)\n# TODO: ship it\n").unwrap();
        std::fs::write(repo.join("node_modules/skip.py"), "# TODO: ignore\n").unwrap();
        std::fs::write(repo.join("notes.txt"), "TODO in a text file\n").unwrap();

        let mut config = TraceLampConfig::default();
        config.skills.directory = skills.display().to_string();
        let findings = collect_skill("todo", repo.to_str().unwrap(), &config).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 2);
        assert_eq!(findings[0].metadata["rule_id"], "skill/todo");
        assert!(findings[0].file.ends_with("app.py"));
    }

    #[test]
    fn inline_ignore_suppresses_a_skill_finding() {
        let dir = tempfile::tempdir().unwrap();
        let skills = dir.path().join("skills");
        std::fs::create_dir_all(&skills).unwrap();
        std::fs::write(
            skills.join("todo.toml"),
            r#"
name = "todo"
description = "flag TODO comments"

[[search_patterns]]
pattern = "TODO"
description = "leftover TODO"
"#,
        )
        .unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(
            repo.join("app.py"),
            "# recon:ignore skill/todo\n# TODO: kept\n",
        )
        .unwrap();
        let mut config = TraceLampConfig::default();
        config.skills.directory = skills.display().to_string();
        let findings = collect_skill("todo", repo.to_str().unwrap(), &config).unwrap();
        assert!(findings.is_empty(), "{findings:?}");
    }
}
