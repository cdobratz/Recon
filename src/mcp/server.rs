use crate::config::ReconConfig;
use anyhow::Result;

/// Serve Recon as an MCP server over stdio
/// TODO(Phase 3): Implement using rmcp ServerHandler with tools:
///   - scan_repo, analyze_file, get_report, list_findings
pub async fn serve_stdio(_config: &ReconConfig) -> Result<()> {
    println!("MCP stdio server is not yet implemented.");
    println!("This will expose scan_repo, analyze_file, get_report, list_findings tools.");
    Ok(())
}

/// Serve Recon as an MCP server over HTTP
/// TODO(Phase 3): Implement using rmcp with hyper_server
pub async fn serve_http(_port: u16, _config: &ReconConfig) -> Result<()> {
    println!("MCP HTTP server is not yet implemented.");
    println!("This will be available in a future release with rmcp integration.");
    Ok(())
}
