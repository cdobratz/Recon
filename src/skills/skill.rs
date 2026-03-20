use serde::{Deserialize, Serialize};

/// A skill definition loaded from a TOML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub description: String,
    /// File patterns this skill applies to (e.g., "*.py", "*.js")
    #[serde(default)]
    pub target_patterns: Vec<String>,
    /// Regex patterns to search for
    #[serde(default)]
    pub search_patterns: Vec<SearchPattern>,
    /// LLM prompt template (uses {context} placeholder)
    #[serde(default)]
    pub llm_prompt: Option<String>,
    /// Severity level for findings from this skill
    #[serde(default = "default_severity")]
    pub severity: String,
    /// Category name for findings
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPattern {
    pub pattern: String,
    pub description: String,
    #[serde(default = "default_severity")]
    pub severity: String,
}

fn default_severity() -> String {
    "info".into()
}
