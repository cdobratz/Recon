use super::Finding;
use crate::parsers::{AstHitKind, ParsedFile};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

struct FunctionPattern {
    regex: Regex,
    lang_filter: Option<&'static str>,
}

static PATTERNS: LazyLock<Vec<FunctionPattern>> = LazyLock::new(|| {
    vec![
        // Python
        FunctionPattern {
            regex: Regex::new(r#"^\s*(?:async\s+)?def\s+(\w+)\s*\((.*?)\)"#).unwrap(),
            lang_filter: Some("Python"),
        },
        // JavaScript/TypeScript
        FunctionPattern {
            regex: Regex::new(r#"^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\((.*?)\)"#).unwrap(),
            lang_filter: Some("JavaScript"),
        },
        FunctionPattern {
            regex: Regex::new(r#"^\s*(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*\((.*?)\)"#).unwrap(),
            lang_filter: Some("TypeScript"),
        },
        // Rust
        FunctionPattern {
            regex: Regex::new(r#"^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*(?:<[^>]*>)?\s*\((.*?)\)"#).unwrap(),
            lang_filter: Some("Rust"),
        },
        // Go
        FunctionPattern {
            regex: Regex::new(r#"^\s*func\s+(?:\([^)]*\)\s+)?(\w+)\s*\((.*?)\)"#).unwrap(),
            lang_filter: Some("Go"),
        },
        // Java
        FunctionPattern {
            regex: Regex::new(r#"^\s*(?:public|private|protected)?\s*(?:static\s+)?(?:\w+\s+)+(\w+)\s*\((.*?)\)\s*(?:throws\s+\w+)?\s*\{"#).unwrap(),
            lang_filter: Some("Java"),
        },
    ]
});

/// Scan a file for function definitions.
///
/// Python, JavaScript, TypeScript, Go, and Rust use the tree-sitter query
/// when the file parsed. Other languages, and files that fail to parse, use regex.
pub fn scan(
    path: &Path,
    source: &str,
    language: &str,
    parsed: Option<&ParsedFile>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let file_str = path.to_string_lossy().to_string();
    let mut seen: HashSet<(usize, String)> = HashSet::new();

    if let Some(parsed) = parsed.filter(|parsed| parsed.parsed) {
        for hit in parsed
            .hits
            .iter()
            .filter(|hit| hit.kind == AstHitKind::Function)
        {
            if !seen.insert((hit.line, hit.name.clone())) {
                continue;
            }
            findings.push(function_finding(
                &file_str,
                language,
                hit.line,
                &hit.snippet,
                &hit.name,
                hit.detail.parse().unwrap_or(0),
                &hit.detail,
                "ast",
            ));
        }
        return findings;
    }

    for (line_num, line) in source.lines().enumerate() {
        for pattern in PATTERNS.iter() {
            // Apply language filter
            if let Some(lang) = pattern.lang_filter {
                if lang != language {
                    continue;
                }
            }

            if let Some(captures) = pattern.regex.captures(line) {
                let fn_name = captures.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                let params = captures.get(2).map(|m| m.as_str()).unwrap_or("");
                let line_no = line_num + 1;
                if !seen.insert((line_no, fn_name.to_string())) {
                    continue;
                }

                let param_count = if params.trim().is_empty() {
                    0
                } else {
                    params.split(',').count()
                };

                findings.push(function_finding(
                    &file_str,
                    language,
                    line_no,
                    line.trim(),
                    fn_name,
                    param_count,
                    params.trim(),
                    "regex",
                ));
            }
        }
    }

    findings
}

#[allow(clippy::too_many_arguments)]
fn function_finding(
    file: &str,
    language: &str,
    line: usize,
    snippet: &str,
    fn_name: &str,
    param_count: usize,
    params: &str,
    detector: &str,
) -> Finding {
    let mut metadata = HashMap::new();
    metadata.insert("function_name".into(), fn_name.into());
    metadata.insert("param_count".into(), param_count.to_string());
    metadata.insert("params".into(), params.into());
    metadata.insert("detector".into(), detector.into());
    let severity = if param_count > 6 { "medium" } else { "info" };
    Finding {
        category: "function".into(),
        severity: severity.into(),
        file: file.into(),
        line,
        title: format!("Function: {fn_name}({param_count} params)"),
        description: format!("Function definition with {param_count} parameters"),
        snippet: snippet.into(),
        language: language.into(),
        metadata,
    }
}
