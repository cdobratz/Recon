//! Baseline files so CI reports only new findings.
//!
//! A fingerprint is the SHA-256 of `rule id + relative path + normalized snippet`.
//! Line numbers are not part of it.

use super::Finding;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

const VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Baseline {
    pub version: u32,
    pub fingerprints: Vec<BaselineEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineEntry {
    pub fingerprint: String,
    pub rule_id: String,
    pub path: String,
    pub snippet: String,
    #[serde(default = "one")]
    pub count: usize,
}

fn one() -> usize {
    1
}

pub fn load(path: &Path) -> Result<Baseline> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read baseline {}", path.display()))?;
    let baseline: Baseline = serde_json::from_str(&text)
        .with_context(|| format!("parse baseline {}", path.display()))?;
    if baseline.version != VERSION {
        anyhow::bail!(
            "baseline {} has version {}, expected {VERSION}",
            path.display(),
            baseline.version
        );
    }
    Ok(baseline)
}

pub fn write(path: &Path, findings: &[Finding], root: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
    }
    let baseline = from_findings(findings, root);
    let text = serde_json::to_string_pretty(&baseline)?;
    std::fs::write(path, text).with_context(|| format!("write baseline {}", path.display()))?;
    eprintln!("Baseline written to: {}", path.display());
    Ok(())
}

pub fn filter(findings: Vec<Finding>, root: &Path, baseline: &Baseline) -> Vec<Finding> {
    let mut remaining: HashMap<String, usize> = HashMap::new();
    for entry in &baseline.fingerprints {
        *remaining.entry(entry.fingerprint.clone()).or_default() += entry.count.max(1);
    }
    let mut kept = Vec::new();
    for finding in findings {
        let fingerprint = fingerprint_for(&finding, root).0;
        let count = remaining.entry(fingerprint).or_insert(0);
        if *count > 0 {
            *count -= 1;
        } else {
            kept.push(finding);
        }
    }
    kept
}

pub fn fingerprint_for(finding: &Finding, root: &Path) -> (String, String, String, String) {
    let rule_id = finding
        .metadata
        .get("rule_id")
        .cloned()
        .unwrap_or_else(|| format!("recon/{}", finding.category));
    let path = relative_path(root, &finding.file);
    let snippet = normalize_snippet(&finding.snippet);
    let fingerprint = digest(&rule_id, &path, &snippet);
    (fingerprint, rule_id, path, snippet)
}

fn from_findings(findings: &[Finding], root: &Path) -> Baseline {
    let mut grouped: HashMap<String, BaselineEntry> = HashMap::new();
    for finding in findings {
        let (fingerprint, rule_id, path, snippet) = fingerprint_for(finding, root);
        grouped
            .entry(fingerprint.clone())
            .and_modify(|entry| entry.count += 1)
            .or_insert(BaselineEntry {
                fingerprint,
                rule_id,
                path,
                snippet,
                count: 1,
            });
    }
    let mut fingerprints: Vec<_> = grouped.into_values().collect();
    fingerprints
        .sort_by(|a, b| (&a.path, &a.rule_id, &a.snippet).cmp(&(&b.path, &b.rule_id, &b.snippet)));
    Baseline {
        version: VERSION,
        fingerprints,
    }
}

pub fn relative_path(root: &Path, file: &str) -> String {
    let path = Path::new(file);
    let rel = path.strip_prefix(root).unwrap_or(path);
    let mut text = rel.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = text.strip_prefix("./") {
        text = stripped.to_string();
    }
    text
}

pub fn normalize_snippet(snippet: &str) -> String {
    snippet.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn digest(rule_id: &str, path: &str, snippet: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(rule_id.as_bytes());
    hasher.update(b"\n");
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    hasher.update(snippet.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn finding(line: usize, snippet: &str) -> Finding {
        let mut metadata = HashMap::new();
        metadata.insert("rule_id".into(), "recon/shell-execution".into());
        Finding {
            category: "agent".into(),
            severity: "high".into(),
            file: "repo/main.go".into(),
            line,
            title: "shell".into(),
            description: "shell".into(),
            snippet: snippet.into(),
            language: "Go".into(),
            metadata,
        }
    }

    #[test]
    fn fingerprint_ignores_line_number_and_indentation() {
        let root = Path::new("repo");
        let first = fingerprint_for(&finding(10, "    exec.Command(\"az\")"), root);
        let second = fingerprint_for(&finding(40, "exec.Command(\"az\")"), root);
        assert_eq!(first.0, second.0);
        assert_eq!(first.2, "main.go");
        let changed = fingerprint_for(&finding(10, "exec.Command(\"aws\")"), root);
        assert_ne!(first.0, changed.0);
    }

    #[test]
    fn filter_keeps_only_new_snippets() {
        let root = Path::new("repo");
        let known = finding(3, "exec.Command(\"az\")");
        let fresh = finding(9, "exec.Command(\"aws\")");
        let baseline = from_findings(std::slice::from_ref(&known), root);
        let kept = filter(vec![known, fresh.clone()], root, &baseline);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].snippet, fresh.snippet);
    }
}
