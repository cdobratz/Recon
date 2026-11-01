//! Recon as an MCP server.
//!
//! stdio is the supported transport. The tools call the static scanner.

use super::tools;
use crate::config::ReconConfig;
use anyhow::Result;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone)]
pub struct ReconMcp {
    config: ReconConfig,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PathParams {
    /// Repository path to read.
    path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListParams {
    /// Repository path to read.
    path: String,
    /// Keep findings of this severity only.
    #[serde(default)]
    severity: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ExplainParams {
    /// Stable rule id, for example recon/shell-execution.
    rule_id: String,
}

#[tool_router]
impl ReconMcp {
    #[tool(
        description = "Statically scan a repository. Does not execute code or open network connections."
    )]
    async fn scan_repo(
        &self,
        Parameters(PathParams { path }): Parameters<PathParams>,
    ) -> Result<String, String> {
        json_tool(tools::scan_repo(&self.config, &path).await)
    }

    #[tool(description = "List static findings, optionally filtered by severity.")]
    async fn list_findings(
        &self,
        Parameters(ListParams { path, severity }): Parameters<ListParams>,
    ) -> Result<String, String> {
        json_tool(tools::list_findings(&self.config, &path, severity.as_deref()).await)
    }

    #[tool(description = "Explain one Recon rule id: help text, ASI mapping, and remediation.")]
    fn explain_finding(
        &self,
        Parameters(ExplainParams { rule_id }): Parameters<ExplainParams>,
    ) -> String {
        serde_json::to_string_pretty(&tools::explain_finding(&rule_id))
            .unwrap_or_else(|err| json!({"error": err.to_string()}).to_string())
    }

    #[tool(
        description = "Return the agent, MCP server, tool, data store, and credential inventory."
    )]
    async fn agent_inventory(
        &self,
        Parameters(PathParams { path }): Parameters<PathParams>,
    ) -> Result<String, String> {
        json_tool(tools::agent_inventory(&self.config, &path).await)
    }
}

#[tool_handler(
    name = "recon",
    instructions = "Static Recon scanner. Tools read files only. They do not spawn processes or open network connections."
)]
impl ServerHandler for ReconMcp {}

fn json_tool(result: Result<serde_json::Value>) -> Result<String, String> {
    match result {
        Ok(value) => serde_json::to_string_pretty(&value).map_err(|err| err.to_string()),
        Err(err) => Err(err.to_string()),
    }
}

/// Serve MCP over stdin and stdout. Log lines go to stderr so they stay off the protocol stream.
pub async fn serve_stdio(config: &ReconConfig) -> Result<()> {
    eprintln!("recon mcp server (stdio)");
    let running = ReconMcp {
        config: config.clone(),
    }
    .serve(rmcp::transport::stdio())
    .await
    .map_err(|err| anyhow::anyhow!("mcp serve: {err}"))?;
    running
        .waiting()
        .await
        .map_err(|err| anyhow::anyhow!("mcp serve: {err}"))?;
    Ok(())
}

/// HTTP transport is not part of this server.
pub async fn serve_http(_port: u16, _config: &ReconConfig) -> Result<()> {
    anyhow::bail!(
        "HTTP MCP transport is not implemented. Use `recon mcp serve --transport stdio`."
    );
}
