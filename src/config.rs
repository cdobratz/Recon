use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level configuration for Recon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconConfig {
    #[serde(default)]
    pub scan: ScanConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub skills: SkillsConfig,
    #[serde(default)]
    pub report: ReportConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// File extensions to scan (empty = all supported)
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Glob patterns for paths to ignore
    #[serde(default = "default_ignore_patterns")]
    pub ignore_patterns: Vec<String>,
    /// Maximum file size in bytes to scan
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
    /// Scan categories to enable
    #[serde(default = "default_scan_categories")]
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider name: ollama, anthropic, openai, mistral
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Model name (provider-specific)
    #[serde(default)]
    pub model: Option<String>,
    /// API key (for paid providers)
    #[serde(default)]
    pub api_key: Option<String>,
    /// Base URL override
    #[serde(default)]
    pub base_url: Option<String>,
    /// Max tokens for LLM responses
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Temperature for LLM responses
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// List of MCP servers to connect to
    #[serde(default)]
    pub servers: Vec<McpServerEntry>,
    /// Transport for serving: stdio, http
    #[serde(default = "default_transport")]
    pub serve_transport: String,
    /// Port for HTTP transport
    #[serde(default = "default_port")]
    pub serve_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    pub name: String,
    pub uri: String,
    #[serde(default = "default_transport")]
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    /// Directory to load skills from
    #[serde(default = "default_skills_dir")]
    pub directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    /// Default output format
    #[serde(default = "default_report_format")]
    pub default_format: String,
    /// Minimum severity to include
    #[serde(default = "default_min_severity")]
    pub min_severity: String,
}

// Default value functions
fn default_ignore_patterns() -> Vec<String> {
    vec![
        "node_modules/**".into(),
        ".git/**".into(),
        "target/**".into(),
        "__pycache__/**".into(),
        ".venv/**".into(),
        "vendor/**".into(),
        "dist/**".into(),
        "build/**".into(),
    ]
}

fn default_max_file_size() -> u64 {
    1_048_576 // 1MB
}

fn default_scan_categories() -> Vec<String> {
    vec![
        "api_endpoints".into(),
        "functions".into(),
        "db_connections".into(),
        "agents".into(),
        "secrets".into(),
    ]
}

fn default_provider() -> String {
    "ollama".into()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_temperature() -> f32 {
    0.3
}

fn default_transport() -> String {
    "stdio".into()
}

fn default_port() -> u16 {
    3000
}

fn default_skills_dir() -> String {
    "skills".into()
}

fn default_report_format() -> String {
    "terminal".into()
}

fn default_min_severity() -> String {
    "info".into()
}

// Trait implementations for defaults
impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            extensions: vec![],
            ignore_patterns: default_ignore_patterns(),
            max_file_size: default_max_file_size(),
            categories: default_scan_categories(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: None,
            api_key: None,
            base_url: None,
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
        }
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            servers: vec![],
            serve_transport: default_transport(),
            serve_port: default_port(),
        }
    }
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            directory: default_skills_dir(),
        }
    }
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            default_format: default_report_format(),
            min_severity: default_min_severity(),
        }
    }
}

impl Default for ReconConfig {
    fn default() -> Self {
        Self {
            scan: ScanConfig::default(),
            llm: LlmConfig::default(),
            mcp: McpConfig::default(),
            skills: SkillsConfig::default(),
            report: ReportConfig::default(),
        }
    }
}

impl ReconConfig {
    /// Load config from file, falling back to defaults
    pub fn load(path: Option<&str>) -> Result<Self> {
        let config_path = match path {
            Some(p) => PathBuf::from(p),
            None => Self::default_config_path(),
        };

        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config: {}", config_path.display()))?;
            let config: ReconConfig = toml::from_str(&contents)
                .with_context(|| format!("Failed to parse config: {}", config_path.display()))?;
            Ok(config)
        } else {
            tracing::debug!("No config file found, using defaults");
            Ok(Self::default())
        }
    }

    /// Default config file path
    pub fn default_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("recon")
            .join("config.toml")
    }
}

/// Set a config value by dot-separated key
pub fn set_value(key: &str, value: &str) -> Result<()> {
    let config_path = ReconConfig::default_config_path();
    let mut config = if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)?;
        toml::from_str(&contents)?
    } else {
        ReconConfig::default()
    };

    match key {
        "llm.provider" => config.llm.provider = value.to_string(),
        "llm.model" => config.llm.model = Some(value.to_string()),
        "llm.api_key" => config.llm.api_key = Some(value.to_string()),
        "llm.base_url" => config.llm.base_url = Some(value.to_string()),
        "report.default_format" => config.report.default_format = value.to_string(),
        "report.min_severity" => config.report.min_severity = value.to_string(),
        _ => anyhow::bail!("Unknown config key: {}", key),
    }

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let toml_str = toml::to_string_pretty(&config)?;
    std::fs::write(&config_path, toml_str)?;
    println!("Set {} = {}", key, value);
    Ok(())
}

/// Get a config value by dot-separated key
pub fn get_value(key: &str) -> Result<()> {
    let config = ReconConfig::load(None)?;
    let value = match key {
        "llm.provider" => config.llm.provider,
        "llm.model" => config.llm.model.unwrap_or_default(),
        "llm.base_url" => config.llm.base_url.unwrap_or_default(),
        "report.default_format" => config.report.default_format,
        "report.min_severity" => config.report.min_severity,
        _ => anyhow::bail!("Unknown config key: {}", key),
    };
    println!("{} = {}", key, value);
    Ok(())
}

/// Show full configuration
pub fn show_config(config: &ReconConfig) -> Result<()> {
    let toml_str = toml::to_string_pretty(config)?;
    println!("{}", toml_str);
    Ok(())
}

/// Initialize default config file
pub fn init_config() -> Result<()> {
    let config_path = ReconConfig::default_config_path();
    if config_path.exists() {
        println!("Config already exists at: {}", config_path.display());
        return Ok(());
    }
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let config = ReconConfig::default();
    let toml_str = toml::to_string_pretty(&config)?;
    std::fs::write(&config_path, &toml_str)?;
    println!("Config initialized at: {}", config_path.display());
    Ok(())
}
