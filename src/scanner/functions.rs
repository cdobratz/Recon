use super::Finding;
use regex::Regex;
use std::collections::HashMap;
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

/// Scan a file for function definitions
pub fn scan(path: &Path, source: &str, language: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let file_str = path.to_string_lossy().to_string();

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

                let param_count = if params.trim().is_empty() {
                    0
                } else {
                    params.split(',').count()
                };

                let mut metadata = HashMap::new();
                metadata.insert("function_name".into(), fn_name.into());
                metadata.insert("param_count".into(), param_count.to_string());
                metadata.insert("params".into(), params.trim().into());

                // Flag functions with many parameters as a code smell
                let severity = if param_count > 6 {
                    "medium"
                } else {
                    "info"
                };

                findings.push(Finding {
                    category: "function".into(),
                    severity: severity.into(),
                    file: file_str.clone(),
                    line: line_num + 1,
                    title: format!("Function: {}({} params)", fn_name, param_count),
                    description: format!("Function definition with {} parameters", param_count),
                    snippet: line.trim().to_string(),
                    language: language.into(),
                    metadata,
                });
            }
        }
    }

    findings
}
