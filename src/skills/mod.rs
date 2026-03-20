pub mod skill;

use crate::config::ReconConfig;
use anyhow::Result;
use skill::SkillDefinition;
use std::path::Path;

/// Add a skill from a TOML file
pub fn add_skill(path: &str, config: &ReconConfig) -> Result<()> {
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
pub fn list_skills(config: &ReconConfig) -> Result<()> {
    let skills_dir = Path::new(&config.skills.directory);
    if !skills_dir.exists() {
        println!("No skills directory found. Run 'recon skill add <path>' to add skills.");
        return Ok(());
    }

    let mut count = 0;
    for entry in std::fs::read_dir(skills_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(skill) = toml::from_str::<SkillDefinition>(&contents) {
                    println!("  {} - {}", skill.name, skill.description);
                    count += 1;
                }
            }
        }
    }

    if count == 0 {
        println!("No skills registered.");
    } else {
        println!("\n{} skill(s) registered.", count);
    }
    Ok(())
}

/// Remove a skill by name
pub fn remove_skill(name: &str, config: &ReconConfig) -> Result<()> {
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
pub fn load_skills(config: &ReconConfig) -> Result<Vec<SkillDefinition>> {
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
