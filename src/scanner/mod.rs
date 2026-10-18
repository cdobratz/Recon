//! Static repository scan.
//!
//! `run_scan` reads files and applies in-process rules. It does not spawn
//! processes and it does not open network connections. MCP config URLs are
//! recorded as text only.

pub mod agents;
pub mod api_endpoints;
pub mod baseline;
pub mod db_connections;
pub mod execution;
pub mod functions;
pub mod mcp_inventory;
pub mod rules;
pub mod secrets;
pub mod suppress;
pub mod walk;

use crate::config::ReconConfig;
use crate::parsers::ParserRegistry;
use anyhow::Result;
use mcp_inventory::InventoryGraph;
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    /// Agent → MCP server/tool → data store → credential graph.
    #[serde(default)]
    pub inventory: InventoryGraph,
    /// Mermaid `flowchart` for `inventory`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub inventory_mermaid: String,
    /// Set when `--baseline` is used and at least one finding is not in it.
    #[serde(skip)]
    pub fail_on_new: bool,
}

/// Options for one `recon scan` invocation.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScanOptions<'a> {
    pub only: Option<&'a [String]>,
    pub ignore: Option<&'a [String]>,
    pub baseline: Option<&'a Path>,
    pub write_baseline: Option<&'a Path>,
}

/// Run all enabled scanners against a repository path.
pub async fn run_scan(
    repo_path: &str,
    config: &ReconConfig,
    options: ScanOptions<'_>,
) -> Result<ScanResults> {
    let start = std::time::Instant::now();
    let mut findings = Vec::new();
    let mut inventory = InventoryGraph::default();
    let mut files_scanned = 0;
    let mut parser_registry = ParserRegistry::new();

    let categories: Vec<&str> = match options.only {
        Some(cats) => cats.iter().map(|s| s.as_str()).collect(),
        None => config.scan.categories.iter().map(|s| s.as_str()).collect(),
    };

    let ignore_patterns: Vec<&str> = match options.ignore {
        Some(pats) => pats.iter().map(|s| s.as_str()).collect(),
        None => config
            .scan
            .ignore_patterns
            .iter()
            .map(|s| s.as_str())
            .collect(),
    };

    let repo = Path::new(repo_path);
    let skipped_outputs = [options.baseline, options.write_baseline];
    for path in walk::list_files(repo, &ignore_patterns)? {
        if skipped_outputs
            .iter()
            .flatten()
            .any(|skip| same_file(&path, skip))
        {
            continue;
        }
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > config.scan.max_file_size {
                continue;
            }
        }
        if walk::is_binary_extension(&path) {
            continue;
        }

        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        if walk::looks_binary(&bytes) {
            continue;
        }
        let source = match String::from_utf8(bytes) {
            Ok(source) => source,
            Err(_) => continue,
        };

        let parsed = parser_registry.parse_file(&path, &source)?;
        let lang_name = parsed
            .as_ref()
            .map(|p| p.language.name())
            .unwrap_or("unknown");

        files_scanned += 1;

        let mut file_findings = Vec::new();
        if categories.contains(&"api_endpoints") {
            file_findings.extend(api_endpoints::scan(
                &path,
                &source,
                lang_name,
                parsed.as_ref(),
            ));
        }
        if categories.contains(&"functions") {
            file_findings.extend(functions::scan(&path, &source, lang_name, parsed.as_ref()));
        }
        if categories.contains(&"db_connections") {
            file_findings.extend(db_connections::scan(&path, &source, lang_name));
        }
        if categories.contains(&"agents") {
            file_findings.extend(agents::scan(&path, &source, lang_name, parsed.as_ref()));
        }
        if categories.contains(&"secrets") {
            file_findings.extend(secrets::scan(&path, &source, lang_name));
        }
        // Agent and MCP inventory are static parses of config text. They never
        // launch the servers or commands named in those files.
        if categories.iter().any(|c| *c == "agents" || *c == "mcp") {
            let mcp = mcp_inventory::scan(&path, &source, lang_name);
            file_findings.extend(mcp.findings);
            inventory.merge(mcp.inventory);
        }
        findings.extend(suppress::apply(file_findings, &source));
    }

    if let Some(path) = options.write_baseline {
        baseline::write(path, &findings, repo)?;
    }
    let mut fail_on_new = false;
    if let Some(path) = options.baseline {
        let loaded = baseline::load(path)?;
        findings = baseline::filter(findings, repo, &loaded);
        fail_on_new = options.write_baseline.is_none() && !findings.is_empty();
    }

    let duration = start.elapsed().as_millis() as u64;
    let inventory_mermaid = mcp_inventory::to_mermaid(&inventory);
    Ok(ScanResults {
        findings,
        files_scanned,
        scan_duration_ms: duration,
        inventory,
        inventory_mermaid,
        fail_on_new,
    })
}

fn same_file(path: &Path, other: &Path) -> bool {
    if path == other {
        return true;
    }
    match (path.canonicalize(), other.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}
