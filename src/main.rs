mod config;
mod scanner;
mod analysis;
mod parsers;
mod llm;
mod mcp;
mod skills;
mod agents;
mod report;

use clap::{Parser, Subcommand};
use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "recon")]
#[command(about = "AI-powered repo scanner for security, API efficiency, and agent analysis")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to config file
    #[arg(long, global = true)]
    config: Option<String>,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a repository for endpoints, functions, DB connections, agents, and secrets
    Scan {
        /// Path to the repository to scan
        path: String,

        /// Output format
        #[arg(short, long, default_value = "terminal")]
        format: String,

        /// Scan only specific categories
        #[arg(long, value_delimiter = ',')]
        only: Option<Vec<String>>,

        /// Paths to ignore (glob patterns)
        #[arg(long, value_delimiter = ',')]
        ignore: Option<Vec<String>>,
    },

    /// Run AI-powered deep analysis on scan results
    Analyze {
        /// Path to the repository to analyze
        path: String,

        /// LLM provider to use
        #[arg(long, default_value = "ollama")]
        provider: String,

        /// Model name
        #[arg(long)]
        model: Option<String>,

        /// Analysis categories
        #[arg(long, value_delimiter = ',')]
        categories: Option<Vec<String>>,
    },

    /// Generate a report from scan/analysis results
    Report {
        /// Path to the repository
        path: String,

        /// Output format: json, sarif, terminal
        #[arg(short, long, default_value = "terminal")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Manage skills (pluggable analysis modules)
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },

    /// Manage and run agents (autonomous analysis workflows)
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },

    /// MCP server/client management
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum SkillAction {
    /// Add a skill from a TOML definition file
    Add { path: String },
    /// List all registered skills
    List,
    /// Remove a skill by name
    Remove { name: String },
}

#[derive(Subcommand)]
enum AgentAction {
    /// Add an agent from a config file
    Add { config: String },
    /// List all registered agents
    List,
    /// Run an agent against a target
    Run {
        /// Agent name
        name: String,
        /// Target repository path
        #[arg(long)]
        target: String,
    },
}

#[derive(Subcommand)]
enum McpAction {
    /// Start Recon as an MCP server
    Serve {
        /// Transport type: stdio, http
        #[arg(long, default_value = "stdio")]
        transport: String,
        /// Port for HTTP transport
        #[arg(long, default_value = "3000")]
        port: u16,
    },
    /// Connect to an external MCP server
    Connect {
        /// MCP server URI
        uri: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Set a configuration value
    Set {
        /// Config key (dot-separated path)
        key: String,
        /// Config value
        value: String,
    },
    /// Get a configuration value
    Get {
        /// Config key (dot-separated path)
        key: String,
    },
    /// Show full configuration
    Show,
    /// Initialize default config file
    Init,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()))
        .init();

    // Load configuration
    let cfg = config::ReconConfig::load(cli.config.as_deref())?;

    match cli.command {
        Commands::Scan { path, format, only, ignore } => {
            tracing::info!("Scanning repository: {}", path);
            let findings = scanner::run_scan(&path, &cfg, only.as_deref(), ignore.as_deref()).await?;
            report::output(&findings, &format, None)?;
        }
        Commands::Analyze { path, provider, model, categories } => {
            tracing::info!("Analyzing repository: {}", path);
            let findings = scanner::run_scan(&path, &cfg, None, None).await?;
            let results = analysis::run_analysis(&findings, &provider, model.as_deref(), categories.as_deref(), &cfg).await?;
            report::output_analysis(&results, "terminal", None)?;
        }
        Commands::Report { path, format, output } => {
            tracing::info!("Generating report for: {}", path);
            let findings = scanner::run_scan(&path, &cfg, None, None).await?;
            report::output(&findings, &format, output.as_deref())?;
        }
        Commands::Skill { action } => match action {
            SkillAction::Add { path } => skills::add_skill(&path, &cfg)?,
            SkillAction::List => skills::list_skills(&cfg)?,
            SkillAction::Remove { name } => skills::remove_skill(&name, &cfg)?,
        },
        Commands::Agent { action } => match action {
            AgentAction::Add { config: agent_cfg } => agents::add_agent(&agent_cfg, &cfg)?,
            AgentAction::List => agents::list_agents(&cfg)?,
            AgentAction::Run { name, target } => {
                agents::run_agent(&name, &target, &cfg).await?;
            }
        },
        Commands::Mcp { action } => match action {
            McpAction::Serve { transport, port } => {
                mcp::serve(&transport, port, &cfg).await?;
            }
            McpAction::Connect { uri } => {
                mcp::connect(&uri, &cfg).await?;
            }
        },
        Commands::Config { action } => match action {
            ConfigAction::Set { key, value } => config::set_value(&key, &value)?,
            ConfigAction::Get { key } => config::get_value(&key)?,
            ConfigAction::Show => config::show_config(&cfg)?,
            ConfigAction::Init => config::init_config()?,
        },
    }

    Ok(())
}
