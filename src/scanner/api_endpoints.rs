use super::Finding;
use crate::parsers::{AstHitKind, ParsedFile};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

/// Patterns for detecting API endpoint definitions across languages.
struct EndpointPattern {
    regex: Regex,
    description: &'static str,
    framework: &'static str,
    /// Languages whose tree-sitter queries replace this pattern.
    replaced_by_ast: &'static [&'static str],
}

static PATTERNS: LazyLock<Vec<EndpointPattern>> = LazyLock::new(|| {
    vec![
        // Python - Flask/FastAPI/Django
        EndpointPattern {
            regex: Regex::new(r#"@(?:app|router|api)\.(get|post|put|delete|patch|options|head)\s*\(\s*['"](.*?)['"]\s*"#).unwrap(),
            description: "Flask/FastAPI route decorator",
            framework: "Flask/FastAPI",
            replaced_by_ast: &["Python"],
        },
        EndpointPattern {
            regex: Regex::new(r#"path\s*\(\s*['"]([^'"]+)['"]\s*,\s*(\w+)"#).unwrap(),
            description: "Django URL path",
            framework: "Django",
            replaced_by_ast: &["Python"],
        },
        // JavaScript/TypeScript - Express
        EndpointPattern {
            regex: Regex::new(r#"(?:app|router)\.(get|post|put|delete|patch|all)\s*\(\s*['"](.*?)['"]\s*"#).unwrap(),
            description: "Express.js route",
            framework: "Express",
            replaced_by_ast: &["JavaScript", "TypeScript"],
        },
        // Go - net/http, gin, echo
        EndpointPattern {
            regex: Regex::new(r#"(?:HandleFunc|Handle|GET|POST|PUT|DELETE|PATCH)\s*\(\s*"([^"]+)""#).unwrap(),
            description: "Go HTTP handler",
            framework: "Go/net-http",
            replaced_by_ast: &["Go"],
        },
        // Java - Spring. No Java grammar; regex is the detector.
        EndpointPattern {
            regex: Regex::new(r#"@(?:Get|Post|Put|Delete|Patch|Request)Mapping\s*\(\s*(?:value\s*=\s*)?['"](.*?)['"]\s*"#).unwrap(),
            description: "Spring request mapping",
            framework: "Spring",
            replaced_by_ast: &[],
        },
        // Rust - actix-web, axum
        EndpointPattern {
            regex: Regex::new(r#"(?:web::|\.route\()\s*['"](.*?)['"]\s*"#).unwrap(),
            description: "Rust web framework route",
            framework: "actix/axum",
            replaced_by_ast: &["Rust"],
        },
        // GraphQL
        EndpointPattern {
            regex: Regex::new(r#"(?:type\s+(?:Query|Mutation|Subscription)\s*\{|@(?:query|mutation|subscription))"#).unwrap(),
            description: "GraphQL schema definition",
            framework: "GraphQL",
            replaced_by_ast: &[],
        },
        // gRPC
        EndpointPattern {
            regex: Regex::new(r#"(?:service\s+\w+\s*\{|rpc\s+\w+\s*\()"#).unwrap(),
            description: "gRPC service definition",
            framework: "gRPC",
            replaced_by_ast: &[],
        },
        // OpenAPI/Swagger references
        EndpointPattern {
            regex: Regex::new(r#"(?:swagger|openapi)\s*:\s*['"]?[23]\."#).unwrap(),
            description: "OpenAPI/Swagger specification",
            framework: "OpenAPI",
            replaced_by_ast: &[],
        },
    ]
});

/// Scan a file for API endpoint definitions.
pub fn scan(
    path: &Path,
    source: &str,
    language: &str,
    parsed: Option<&ParsedFile>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let file_str = path.to_string_lossy().to_string();
    let mut seen_lines = HashSet::new();
    let ast_parsed = parsed.map(|parsed| parsed.parsed).unwrap_or(false);

    if let Some(parsed) = parsed.filter(|parsed| parsed.parsed) {
        for hit in parsed
            .hits
            .iter()
            .filter(|hit| hit.kind == AstHitKind::Endpoint)
        {
            if !seen_lines.insert(hit.line) {
                continue;
            }
            let framework = endpoint_framework(language, &hit.detail);
            findings.push(endpoint_finding(
                &file_str,
                language,
                hit.line,
                &hit.snippet,
                &hit.name,
                framework,
                endpoint_description(framework),
                "ast",
            ));
        }
    }

    for (line_num, line) in source.lines().enumerate() {
        let line_no = line_num + 1;
        for pattern in PATTERNS.iter() {
            if ast_parsed && pattern.replaced_by_ast.contains(&language) {
                continue;
            }
            if seen_lines.contains(&line_no) {
                continue;
            }
            if let Some(captures) = pattern.regex.captures(line) {
                let matched = captures.get(0).map(|m| m.as_str()).unwrap_or("");
                let endpoint = captures
                    .get(1)
                    .or(captures.get(2))
                    .map(|m| m.as_str())
                    .unwrap_or(matched);
                seen_lines.insert(line_no);
                findings.push(endpoint_finding(
                    &file_str,
                    language,
                    line_no,
                    line.trim(),
                    endpoint,
                    pattern.framework,
                    pattern.description,
                    "regex",
                ));
            }
        }
    }

    findings
}

fn endpoint_framework(language: &str, method: &str) -> &'static str {
    match (language, method) {
        ("Python", "path") => "Django",
        ("Python", _) => "Flask/FastAPI",
        ("JavaScript" | "TypeScript", _) => "Express",
        ("Go", _) => "Go/net-http",
        ("Rust", _) => "actix/axum",
        _ => "unknown",
    }
}

fn endpoint_description(framework: &str) -> &'static str {
    match framework {
        "Django" => "Django URL path",
        "Express" => "Express.js route",
        "Go/net-http" => "Go HTTP handler",
        "actix/axum" => "Rust web framework route",
        _ => "Flask/FastAPI route decorator",
    }
}

#[allow(clippy::too_many_arguments)]
fn endpoint_finding(
    file: &str,
    language: &str,
    line: usize,
    snippet: &str,
    endpoint: &str,
    framework: &str,
    description: &str,
    detector: &str,
) -> Finding {
    let mut metadata = HashMap::new();
    metadata.insert("framework".into(), framework.into());
    metadata.insert("endpoint".into(), endpoint.into());
    metadata.insert("detector".into(), detector.into());
    Finding {
        category: "api_endpoint".into(),
        severity: "info".into(),
        file: file.into(),
        line,
        title: format!("API endpoint: {endpoint}"),
        description: format!("{description} ({framework})"),
        snippet: snippet.into(),
        language: language.into(),
        metadata,
    }
}
