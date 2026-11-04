//! Tool bodies for the TraceLamp MCP server.
//!
//! Each tool calls the in-process scanner. None of them spawn a process or
//! open a network connection.

use crate::config::TraceLampConfig;
use crate::scanner::{self, rules, ScanOptions};
use anyhow::Result;
use serde_json::{json, Value};

pub async fn scan_repo(config: &TraceLampConfig, path: &str) -> Result<Value> {
    let results = scanner::run_scan(path, config, ScanOptions::default()).await?;
    Ok(json!({
        "files_scanned": results.files_scanned,
        "finding_count": results.findings.len(),
        "findings": results.findings.iter().map(compact).collect::<Vec<_>>(),
        "inventory": results.inventory,
    }))
}

pub async fn list_findings(
    config: &TraceLampConfig,
    path: &str,
    severity: Option<&str>,
) -> Result<Value> {
    let results = scanner::run_scan(path, config, ScanOptions::default()).await?;
    let findings: Vec<_> = results
        .findings
        .iter()
        .filter(|finding| match severity {
            Some(wanted) => finding.severity.eq_ignore_ascii_case(wanted),
            None => true,
        })
        .map(compact)
        .collect();
    Ok(json!({
        "files_scanned": results.files_scanned,
        "finding_count": findings.len(),
        "findings": findings,
    }))
}

pub fn explain_finding(rule_id: &str) -> Value {
    match rules::by_id(rule_id) {
        Some(rule) => json!({
            "rule_id": rule.id,
            "name": rule.name,
            "severity": rule.severity,
            "asi": rule.asi,
            "help": rule.help,
            "remediation": rule.remediation,
            "help_uri": rule.help_uri,
        }),
        None => json!({
            "rule_id": rule_id,
            "known": false,
            "help": "No built-in rule uses that id.",
        }),
    }
}

pub async fn agent_inventory(config: &TraceLampConfig, path: &str) -> Result<Value> {
    let only = ["agents".to_string(), "mcp".to_string()];
    let results = scanner::run_scan(
        path,
        config,
        ScanOptions {
            only: Some(&only),
            ..ScanOptions::default()
        },
    )
    .await?;
    Ok(json!({
        "inventory": results.inventory,
        "mermaid": results.inventory_mermaid,
    }))
}

fn compact(finding: &scanner::Finding) -> Value {
    json!({
        "rule_id": finding.metadata.get("rule_id").cloned().unwrap_or_default(),
        "severity": finding.severity,
        "file": finding.file,
        "line": finding.line,
        "title": finding.title,
        "snippet": finding.snippet,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tools_read_a_repo_without_leaving_the_process() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("main.go"),
            "package main\nimport \"os/exec\"\nfunc main() { exec.Command(\"az\").Run() }\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join(".cursor")).unwrap();
        std::fs::write(
            dir.path().join(".cursor/mcp.json"),
            r#"{"mcpServers":{"fs":{"command":"npx","args":["-y","server-filesystem","/"]}}}"#,
        )
        .unwrap();

        let config = TraceLampConfig::default();
        let path = dir.path().to_str().unwrap();
        let scan = scan_repo(&config, path).await.unwrap();
        assert!(scan["finding_count"].as_u64().unwrap() >= 1);

        let highs = list_findings(&config, path, Some("high")).await.unwrap();
        assert!(highs["finding_count"].as_u64().unwrap() >= 1);
        let lows = list_findings(&config, path, Some("low")).await.unwrap();
        assert_eq!(lows["finding_count"].as_u64().unwrap(), 0);

        let explained = explain_finding("recon/shell-execution");
        assert_eq!(explained["severity"], "high");
        assert!(explained["help"].as_str().unwrap().contains("db.Exec"));
        assert_eq!(explain_finding("recon/missing")["known"], false);

        let inventory = agent_inventory(&config, path).await.unwrap();
        assert!(inventory["mermaid"].as_str().unwrap().contains("flowchart"));
    }
}
