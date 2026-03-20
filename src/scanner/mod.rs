pub mod api_endpoints;
pub mod functions;
pub mod db_connections;
pub mod agents;
pub mod secrets;

use crate::config::ReconConfig;
use crate::parsers::ParserRegistry;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

/// A single finding from any scanner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Category: api_endpoint, function, db_connection, agent, secret
    pub category: String,
    /// Severity: critical, high, medium, low, info
    pub severity: String,
    /// File path where the finding was discovered
    pub file: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Short title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// The matched code snippet
    pub snippet: String,
    /// Language of the source file
    pub language: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// Collection of all scan findings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResults {
    pub findings: Vec<Finding>,
    pub files_scanned: usize,
    pub scan_duration_ms: u64,
}

/// Run all enabled scanners against a repository path
pub async fn run_scan(
    repo_path: &str,
    config: &ReconConfig,
    only: Option<&[String]>,
    ignore: Option<&[String]>,
) -> Result<ScanResults> {
    let start = std::time::Instant::now();
    let mut findings = Vec::new();
    let mut files_scanned = 0;
    let mut parser_registry = ParserRegistry::new();

    let categories: Vec<&str> = match only {
        Some(cats) => cats.iter().map(|s| s.as_str()).collect(),
        None => config.scan.categories.iter().map(|s| s.as_str()).collect(),
    };

    let ignore_patterns: Vec<&str> = match ignore {
        Some(pats) => pats.iter().map(|s| s.as_str()).collect(),
        None => config.scan.ignore_patterns.iter().map(|s| s.as_str()).collect(),
    };

    let repo = Path::new(repo_path);
    for entry in WalkDir::new(repo).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Skip ignored paths
        let rel_path = path.strip_prefix(repo).unwrap_or(path);
        let rel_str = rel_path.to_string_lossy();
        if should_ignore(&rel_str, &ignore_patterns) {
            continue;
        }

        // Skip files over size limit
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() > config.scan.max_file_size {
                continue;
            }
        }

        // Read file
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue, // Skip binary files
        };

        // Parse with tree-sitter (if supported language)
        let parsed = parser_registry.parse_file(path, &source)?;
        let lang_name = parsed.as_ref().map(|p| p.language.name()).unwrap_or("unknown");

        files_scanned += 1;

        // Run enabled scanners
        if categories.contains(&"api_endpoints") {
            findings.extend(api_endpoints::scan(path, &source, lang_name));
        }
        if categories.contains(&"functions") {
            findings.extend(functions::scan(path, &source, lang_name));
        }
        if categories.contains(&"db_connections") {
            findings.extend(db_connections::scan(path, &source, lang_name));
        }
        if categories.contains(&"agents") {
            findings.extend(agents::scan(path, &source, lang_name));
        }
        if categories.contains(&"secrets") {
            findings.extend(secrets::scan(path, &source, lang_name));
        }
    }

    let duration = start.elapsed().as_millis() as u64;
    Ok(ScanResults {
        findings,
        files_scanned,
        scan_duration_ms: duration,
    })
}

/// Check if a relative path matches any ignore pattern
fn should_ignore(rel_path: &str, patterns: &[&str]) -> bool {
    let normalized = rel_path.replace('\\', "/");
    for pattern in patterns {
        let pat = pattern.trim_end_matches("/**");
        if normalized.starts_with(pat) || normalized.contains(&format!("/{}/", pat)) {
            return true;
        }
    }
    false
}
