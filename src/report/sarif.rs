use crate::scanner::rules;
use crate::scanner::ScanResults;
use anyhow::Result;
use serde_json::{json, Value};

/// Format scan results as SARIF (Static Analysis Results Interchange Format)
pub fn format_scan(results: &ScanResults) -> Result<String> {
    let sarif_results: Vec<Value> = results
        .findings
        .iter()
        .map(|finding| {
            let rule_id = finding
                .metadata
                .get("rule_id")
                .cloned()
                .unwrap_or_else(|| format!("recon/{}", finding.category));
            let mut text = finding.description.clone();
            if let Some(remediation) = finding.metadata.get("remediation") {
                if !text.contains(remediation) {
                    text.push(' ');
                    text.push_str(remediation);
                }
            }
            json!({
                "ruleId": rule_id,
                "level": severity_to_sarif_level(&finding.severity),
                "message": { "text": text },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": finding.file },
                        "region": { "startLine": finding.line }
                    }
                }]
            })
        })
        .collect();

    let sarif_rules: Vec<Value> = rules::catalog()
        .iter()
        .map(|rule| {
            json!({
                "id": rule.id,
                "name": rule.name,
                "shortDescription": { "text": rule.short_description },
                "fullDescription": { "text": rule.help },
                "help": {
                    "text": rule.help,
                    "markdown": rule.help
                },
                "helpUri": rule.help_uri,
                "defaultConfiguration": {
                    "level": severity_to_sarif_level(rule.severity)
                },
                "properties": {
                    "tags": rule_tags(rule.asi),
                    "security-severity": if rule.severity == "critical" { "9.0" } else { "8.0" },
                    "asi": rule.asi
                }
            })
        })
        .collect();

    let sarif = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "recon",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/cdobratz/Recon",
                    "rules": sarif_rules
                }
            },
            "results": sarif_results
        }]
    });

    Ok(serde_json::to_string_pretty(&sarif)?)
}

fn rule_tags(asi: &[&str]) -> Vec<String> {
    let mut tags = vec!["security".to_string()];
    tags.extend(asi.iter().map(|id| (*id).to_string()));
    tags
}

fn severity_to_sarif_level(severity: &str) -> &str {
    match severity {
        "critical" | "high" => "error",
        "medium" => "warning",
        "low" | "info" => "note",
        _ => "none",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{Finding, ScanResults};
    use std::collections::HashMap;

    fn empty_results() -> ScanResults {
        ScanResults {
            findings: Vec::new(),
            files_scanned: 0,
            scan_duration_ms: 0,
            inventory: Default::default(),
            inventory_mermaid: String::new(),
        }
    }

    #[test]
    fn sarif_lists_stable_rules_with_help_and_asi() {
        let text = format_scan(&empty_results()).unwrap();
        let sarif: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            sarif["runs"][0]["tool"]["driver"]["informationUri"],
            "https://github.com/cdobratz/Recon"
        );
        let rules = sarif["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap();
        for id in [
            "recon/mcp-unpinned-package",
            "recon/mcp-remote-cve-2025-6514",
            "recon/mcp-secret-in-env",
            "recon/mcp-filesystem-root",
            "recon/mcp-shell-without-approval",
            "recon/agent-cloud-spend",
            "recon/shell-execution",
        ] {
            let rule = rules
                .iter()
                .find(|rule| rule["id"] == id)
                .unwrap_or_else(|| panic!("missing {id}"));
            let help = rule["help"]["text"].as_str().unwrap();
            assert!(help.len() > 40, "{id} help too short");
            let tags = rule["properties"]["tags"].to_string();
            assert!(tags.contains("ASI0"), "{id} missing ASI tag: {tags}");
        }
        let cloud = rules
            .iter()
            .find(|rule| rule["id"] == "recon/agent-cloud-spend")
            .unwrap();
        let help_uri = cloud["helpUri"].as_str().unwrap();
        assert!(help_uri.contains("https://github.com/cdobratz/AzGuard"));
        assert_eq!(cloud["properties"]["asi"][0], "ASI02");
        assert_eq!(
            rules
                .iter()
                .find(|rule| rule["id"] == "recon/mcp-unpinned-package")
                .unwrap()["properties"]["asi"][0],
            "ASI04"
        );
        assert_eq!(
            rules
                .iter()
                .find(|rule| rule["id"] == "recon/mcp-shell-without-approval")
                .unwrap()["properties"]["asi"][1],
            "ASI05"
        );
    }

    #[test]
    fn sarif_result_uses_rule_id_and_remediation() {
        let mut metadata = HashMap::new();
        metadata.insert("rule_id".into(), "recon/agent-cloud-spend".into());
        metadata.insert(
            "remediation".into(),
            "See https://github.com/cdobratz/AzGuard#agent-guard".into(),
        );
        let mut results = empty_results();
        results.findings.push(Finding {
            category: "agent".into(),
            severity: "high".into(),
            file: "mcp.json".into(),
            line: 3,
            title: "Agent can spend cloud money".into(),
            description: "This agent can spend cloud money.".into(),
            snippet: "aws ec2 run-instances".into(),
            language: "unknown".into(),
            metadata,
        });
        let text = format_scan(&results).unwrap();
        let sarif: serde_json::Value = serde_json::from_str(&text).unwrap();
        let result = &sarif["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "recon/agent-cloud-spend");
        let message = result["message"]["text"].as_str().unwrap();
        assert!(message.contains("https://github.com/cdobratz/AzGuard#agent-guard"));
        assert!(message.contains("azguard") || message.contains("AzGuard"));
    }
}
