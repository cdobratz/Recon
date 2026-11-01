//! MCP client used by `recon mcp connect`.
//!
//! This spawns the named command. `recon scan` does not call it.

use crate::config::ReconConfig;
use anyhow::{bail, Result};
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::ServiceExt;
use tokio::process::Command;

/// Connect to a stdio MCP server and print its tools.
///
/// `command_line` is a program plus arguments. Quote arguments that contain
/// spaces. `http://` and `https://` URIs are refused here.
pub async fn connect(command_line: &str, _config: &ReconConfig) -> Result<()> {
    if command_line.starts_with("http://") || command_line.starts_with("https://") {
        bail!(
            "HTTP MCP connect is not implemented. Pass a stdio command, for example: \
             recon mcp connect \"recon mcp serve\""
        );
    }
    let mut args = split_command(command_line)?;
    let program = args.remove(0);
    let command = Command::new(&program).configure(|cmd| {
        cmd.args(&args);
    });
    let transport = TokioChildProcess::new(command)?;
    let client = ().serve(transport).await.map_err(|err| anyhow::anyhow!("mcp connect: {err}"))?;
    if let Some(info) = client.peer_info() {
        if let Some(server) = &info.server_info {
            println!("Connected to {} {}", server.name, server.version);
        }
        if let Some(instructions) = &info.instructions {
            println!("{instructions}");
        }
    }
    let tools = client
        .list_all_tools()
        .await
        .map_err(|err| anyhow::anyhow!("list tools: {err}"))?;
    if tools.is_empty() {
        println!("No tools.");
    } else {
        println!("Tools:");
        for tool in &tools {
            let description = tool.description.as_deref().unwrap_or("");
            println!("  {} {}", tool.name, description);
        }
    }
    drop(client);
    Ok(())
}

pub fn split_command(input: &str) -> Result<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    for ch in input.chars() {
        match ch {
            '"' => in_quote = !in_quote,
            c if c.is_whitespace() && !in_quote => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if in_quote {
        bail!("unbalanced quote in command: {input}");
    }
    if !current.is_empty() {
        args.push(current);
    }
    if args.is_empty() {
        bail!("empty MCP command");
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_keep_spaces() {
        let args = split_command(r#"recon mcp serve --config "/tmp/my config.toml""#).unwrap();
        assert_eq!(
            args,
            vec!["recon", "mcp", "serve", "--config", "/tmp/my config.toml"]
        );
    }
}
