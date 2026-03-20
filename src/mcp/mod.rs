pub mod client;
pub mod server;

use crate::config::ReconConfig;
use anyhow::Result;

/// Start Recon as an MCP server
pub async fn serve(transport: &str, port: u16, config: &ReconConfig) -> Result<()> {
    match transport {
        "stdio" => {
            println!("Starting Recon MCP server (stdio transport)...");
            server::serve_stdio(config).await
        }
        "http" => {
            println!("Starting Recon MCP server on port {}...", port);
            server::serve_http(port, config).await
        }
        _ => anyhow::bail!("Unknown transport: {}. Use 'stdio' or 'http'.", transport),
    }
}

/// Connect to an external MCP server
pub async fn connect(uri: &str, config: &ReconConfig) -> Result<()> {
    println!("Connecting to MCP server: {}", uri);
    client::connect(uri, config).await
}
