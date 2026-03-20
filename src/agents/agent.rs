use serde::{Deserialize, Serialize};

/// Agent configuration loaded from a TOML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub description: String,
    /// Agent type: security_auditor, api_optimizer, agent_inspector
    pub agent_type: String,
    /// Skills this agent uses
    #[serde(default)]
    pub skills: Vec<String>,
    /// Analysis categories to run
    #[serde(default)]
    pub categories: Vec<String>,
    /// LLM provider override
    #[serde(default)]
    pub llm_provider: Option<String>,
    /// LLM model override
    #[serde(default)]
    pub llm_model: Option<String>,
    /// MCP servers this agent can invoke
    #[serde(default)]
    pub mcp_servers: Vec<String>,
}
