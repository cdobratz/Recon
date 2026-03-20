use crate::scanner::ScanResults;
use anyhow::Result;
use serde_json::json;

/// Format scan results as SARIF (Static Analysis Results Interchange Format)
pub fn format_scan(results: &ScanResults) -> Result<String> {
    let sarif_results: Vec<serde_json::Value> = results.findings.iter().map(|f| {
        json!({
            "ruleId": format!("recon/{}", f.category),
            "level": severity_to_sarif_level(&f.severity),
            "message": { "text": f.description },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": f.file },
                    "region": { "startLine": f.line }
                }
            }]
        })
    }).collect();

    let sarif = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "recon",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/recon-cli/recon"
                }
            },
            "results": sarif_results
        }]
    });

    Ok(serde_json::to_string_pretty(&sarif)?)
}

fn severity_to_sarif_level(severity: &str) -> &str {
    match severity {
        "critical" | "high" => "error",
        "medium" => "warning",
        "low" | "info" => "note",
        _ => "none",
    }
}
