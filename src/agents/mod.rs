pub mod agent;

use crate::config::ReconConfig;
use agent::AgentConfig;
use anyhow::Result;
use std::path::Path;

/// Add an agent from a config file
pub fn add_agent(config_path: &str, _recon_config: &ReconConfig) -> Result<()> {
    let path = Path::new(config_path);
    if !path.exists() {
        anyhow::bail!("Agent config not found: {}", config_path);
    }

    let contents = std::fs::read_to_string(path)?;
    let agent: AgentConfig = toml::from_str(&contents)?;

    let agents_dir = Path::new("agents");
    std::fs::create_dir_all(agents_dir)?;

    let dest = agents_dir.join(format!("{}.toml", agent.name));
    std::fs::copy(path, &dest)?;

    println!("Added agent: {} -> {}", agent.name, dest.display());
    Ok(())
}

/// List all registered agents
pub fn list_agents(_config: &ReconConfig) -> Result<()> {
    let agents_dir = Path::new("agents");
    if !agents_dir.exists() {
        println!("No agents directory found.");
        return Ok(());
    }

    let mut count = 0;
    for entry in std::fs::read_dir(agents_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(agent) = toml::from_str::<AgentConfig>(&contents) {
                    println!(
                        "  {} - {} [{}]",
                        agent.name, agent.description, agent.agent_type
                    );
                    count += 1;
                }
            }
        }
    }

    if count == 0 {
        println!("No agents registered.");
    } else {
        println!("\n{} agent(s) registered.", count);
    }
    Ok(())
}

/// Run an agent against a target repository.
///
/// The workflow is static scan, then the agent's skills, then rule-based
/// analysis. An LLM call happens only when `use_llm` is set or the agent
/// config sets `use_llm = true`.
pub async fn run_agent(
    name: &str,
    target: &str,
    config: &ReconConfig,
    use_llm: bool,
) -> Result<()> {
    let agent_file = Path::new("agents").join(format!("{name}.toml"));
    if !agent_file.exists() {
        anyhow::bail!("Agent not found: {name}. Run 'recon agent list' to see available agents.");
    }

    let contents = std::fs::read_to_string(&agent_file)?;
    let agent: AgentConfig = toml::from_str(&contents)?;
    execute(&agent, target, config, use_llm || agent.use_llm).await?;
    Ok(())
}

pub async fn execute(
    agent: &AgentConfig,
    target: &str,
    config: &ReconConfig,
    use_llm: bool,
) -> Result<crate::analysis::AnalysisResults> {
    println!("Running agent '{}' against target: {}", agent.name, target);

    let mut scan =
        crate::scanner::run_scan(target, config, crate::scanner::ScanOptions::default()).await?;
    let scanned = scan.findings.len();
    for skill_name in &agent.skills {
        let extra = crate::skills::collect_skill(skill_name, target, config)?;
        println!("Skill {skill_name} added {} finding(s).", extra.len());
        scan.findings.extend(extra);
    }
    println!(
        "Scan produced {scanned} finding(s); {} after skills.",
        scan.findings.len()
    );

    let provider = agent
        .llm_provider
        .as_deref()
        .unwrap_or(config.llm.provider.as_str());
    let model = agent.llm_model.as_deref().or(config.llm.model.as_deref());
    let categories = if agent.categories.is_empty() {
        None
    } else {
        Some(agent.categories.as_slice())
    };
    let results = crate::analysis::run_analysis(
        &scan,
        config,
        crate::analysis::AnalysisRequest {
            provider,
            model,
            categories,
            use_llm,
            max_cost_usd: None,
        },
    )
    .await?;
    crate::report::output_analysis(&results, "terminal", None)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn agent_applies_a_skill_without_calling_a_model() {
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
severity = "info"
"#,
        )
        .unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(repo.join("app.py"), "# TODO: later\n").unwrap();

        let mut config = ReconConfig::default();
        config.skills.directory = skills.display().to_string();
        let agent = AgentConfig {
            name: "reviewer".into(),
            description: "local review".into(),
            agent_type: "security_auditor".into(),
            skills: vec!["todo".into()],
            categories: vec!["security".into()],
            llm_provider: None,
            llm_model: None,
            mcp_servers: Vec::new(),
            use_llm: false,
        };
        let results = execute(&agent, repo.to_str().unwrap(), &config, false)
            .await
            .unwrap();
        let security = results.security.expect("security report");
        assert!(security.llm_summary.is_none());
        assert!(security.details.contains("Scanned 1 file"));

        let mut missing = agent.clone();
        missing.skills = vec!["missing-skill".into()];
        assert!(execute(&missing, repo.to_str().unwrap(), &config, false)
            .await
            .is_err());
    }
}
