use super::Finding;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

/// Patterns for detecting API endpoint definitions across languages
struct EndpointPattern {
    regex: Regex,
    description: &'static str,
    framework: &'static str,
}

static PATTERNS: LazyLock<Vec<EndpointPattern>> = LazyLock::new(|| {
    vec![
        // Python - Flask/FastAPI/Django
        EndpointPattern {
            regex: Regex::new(r#"@(?:app|router|api)\.(get|post|put|delete|patch|options|head)\s*\(\s*['"](.*?)['"]\s*"#).unwrap(),
            description: "Flask/FastAPI route decorator",
            framework: "Flask/FastAPI",
        },
        EndpointPattern {
            regex: Regex::new(r#"path\s*\(\s*['"]([^'"]+)['"]\s*,\s*(\w+)"#).unwrap(),
            description: "Django URL path",
            framework: "Django",
        },
        // JavaScript/TypeScript - Express
        EndpointPattern {
            regex: Regex::new(r#"(?:app|router)\.(get|post|put|delete|patch|all)\s*\(\s*['"](.*?)['"]\s*"#).unwrap(),
            description: "Express.js route",
            framework: "Express",
        },
        // Go - net/http, gin, echo
        EndpointPattern {
            regex: Regex::new(r#"(?:HandleFunc|Handle|GET|POST|PUT|DELETE|PATCH)\s*\(\s*"([^"]+)""#).unwrap(),
            description: "Go HTTP handler",
            framework: "Go/net-http",
        },
        // Java - Spring
        EndpointPattern {
            regex: Regex::new(r#"@(?:Get|Post|Put|Delete|Patch|Request)Mapping\s*\(\s*(?:value\s*=\s*)?['"](.*?)['"]\s*"#).unwrap(),
            description: "Spring request mapping",
            framework: "Spring",
        },
        // Rust - actix-web, axum
        EndpointPattern {
            regex: Regex::new(r#"(?:web::|\.route\()\s*['"](.*?)['"]\s*"#).unwrap(),
            description: "Rust web framework route",
            framework: "actix/axum",
        },
        // GraphQL
        EndpointPattern {
            regex: Regex::new(r#"(?:type\s+(?:Query|Mutation|Subscription)\s*\{|@(?:query|mutation|subscription))"#).unwrap(),
            description: "GraphQL schema definition",
            framework: "GraphQL",
        },
        // gRPC
        EndpointPattern {
            regex: Regex::new(r#"(?:service\s+\w+\s*\{|rpc\s+\w+\s*\()"#).unwrap(),
            description: "gRPC service definition",
            framework: "gRPC",
        },
        // OpenAPI/Swagger references
        EndpointPattern {
            regex: Regex::new(r#"(?:swagger|openapi)\s*:\s*['"]?[23]\."#).unwrap(),
            description: "OpenAPI/Swagger specification",
            framework: "OpenAPI",
        },
    ]
});

/// Scan a file for API endpoint definitions
pub fn scan(path: &Path, source: &str, language: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let file_str = path.to_string_lossy().to_string();

    for (line_num, line) in source.lines().enumerate() {
        for pattern in PATTERNS.iter() {
            if let Some(captures) = pattern.regex.captures(line) {
                let matched = captures.get(0).map(|m| m.as_str()).unwrap_or("");
                let endpoint = captures.get(1).or(captures.get(2))
                    .map(|m| m.as_str())
                    .unwrap_or(matched);

                let mut metadata = HashMap::new();
                metadata.insert("framework".into(), pattern.framework.into());
                metadata.insert("endpoint".into(), endpoint.into());

                findings.push(Finding {
                    category: "api_endpoint".into(),
                    severity: "info".into(),
                    file: file_str.clone(),
                    line: line_num + 1,
                    title: format!("API endpoint: {}", endpoint),
                    description: format!("{} ({})", pattern.description, pattern.framework),
                    snippet: line.trim().to_string(),
                    language: language.into(),
                    metadata,
                });
            }
        }
    }

    findings
}
