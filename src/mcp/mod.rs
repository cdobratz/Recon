pub mod client;
pub mod server;
pub mod tools;

use crate::config::TraceLampConfig;
use anyhow::Result;

/// Start TraceLamp as an MCP server
pub async fn serve(transport: &str, port: u16, config: &TraceLampConfig) -> Result<()> {
    match transport {
        "stdio" => server::serve_stdio(config).await,
        "http" => server::serve_http(port, config).await,
        _ => anyhow::bail!("Unknown transport: {}. Use 'stdio' or 'http'.", transport),
    }
}

/// Connect to an external MCP server
pub async fn connect(uri: &str, config: &TraceLampConfig) -> Result<()> {
    println!("Connecting to MCP server: {}", uri);
    client::connect(uri, config).await
}
