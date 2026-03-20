pub mod agent;

use crate::config::ReconConfig;
use anyhow::Result;
use agent::AgentConfig;
use std::path::Path;

/// Add an agent from a config file
pub fn add_agent(config_path: &str, recon_config: &ReconConfig) -> Result<()> {
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
                    println!("  {} - {} [{}]", agent.name, agent.description, agent.agent_type);
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

/// Run an agent against a target repository
pub async fn run_agent(name: &str, target: &str, config: &ReconConfig) -> Result<()> {
    let agent_file = Path::new("agents").join(format!("{}.toml", name));
    if !agent_file.exists() {
        anyhow::bail!("Agent not found: {}. Run 'recon agent list' to see available agents.", name);
    }

    let contents = std::fs::read_to_string(&agent_file)?;
    let agent: AgentConfig = toml::from_str(&contents)?;

    println!("Running agent '{}' against target: {}", agent.name, target);

    // Phase 4: Full agent workflow (scan -> analyze -> report)
    let findings = crate::scanner::run_scan(target, config, None, None).await?;
    let results = crate::analysis::run_analysis(&findings, &config.llm.provider, None, None, config).await?;
    crate::report::output_analysis(&results, "terminal", None)?;

    Ok(())
}
