use crate::config::ReconConfig;
use anyhow::Result;

/// Connect to an external MCP server and list available tools
/// TODO(Phase 3): Implement using rmcp crate
pub async fn connect(uri: &str, _config: &ReconConfig) -> Result<()> {
    println!("MCP client connection to '{}' is not yet implemented.", uri);
    println!("This will be available in a future release with rmcp integration.");
    Ok(())
}
