use crate::scanner::ScanResults;
use anyhow::Result;

pub fn format_scan(results: &ScanResults) -> Result<String> {
    Ok(serde_json::to_string_pretty(results)?)
}
