use super::Finding;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

struct SecretPattern {
    regex: Regex,
    secret_type: &'static str,
    description: &'static str,
}

static PATTERNS: LazyLock<Vec<SecretPattern>> = LazyLock::new(|| {
    vec![
        // AWS
        SecretPattern {
            regex: Regex::new(r#"(?i)AKIA[0-9A-Z]{16}"#).unwrap(),
            secret_type: "AWS Access Key",
            description: "AWS Access Key ID detected",
        },
        SecretPattern {
            regex: Regex::new(r#"(?i)(?:aws_secret_access_key|aws_secret)\s*[=:]\s*['"]?[A-Za-z0-9/+=]{40}['"]?"#).unwrap(),
            secret_type: "AWS Secret Key",
            description: "AWS Secret Access Key detected",
        },
        // GitHub
        SecretPattern {
            regex: Regex::new(r#"ghp_[A-Za-z0-9]{36}"#).unwrap(),
            secret_type: "GitHub Token",
            description: "GitHub Personal Access Token detected",
        },
        SecretPattern {
            regex: Regex::new(r#"github_pat_[A-Za-z0-9]{22}_[A-Za-z0-9]{59}"#).unwrap(),
            secret_type: "GitHub Fine-Grained Token",
            description: "GitHub Fine-Grained Personal Access Token detected",
        },
        // Google
        SecretPattern {
            regex: Regex::new(r#"AIza[0-9A-Za-z\-_]{35}"#).unwrap(),
            secret_type: "Google API Key",
            description: "Google API Key detected",
        },
        // Stripe
        SecretPattern {
            regex: Regex::new(r#"(?:sk|pk)_(?:test|live)_[A-Za-z0-9]{24,}"#).unwrap(),
            secret_type: "Stripe Key",
            description: "Stripe API Key detected",
        },
        // Slack
        SecretPattern {
            regex: Regex::new(r#"xox[baprs]-[0-9]{10,}-[A-Za-z0-9]+"#).unwrap(),
            secret_type: "Slack Token",
            description: "Slack Token detected",
        },
        // Generic API keys
        SecretPattern {
            regex: Regex::new(r#"(?i)(?:api_key|apikey|api_secret|api_token)\s*[=:]\s*['"][A-Za-z0-9\-_]{20,}['"]"#).unwrap(),
            secret_type: "Generic API Key",
            description: "Generic API key/secret assignment detected",
        },
        // Private keys
        SecretPattern {
            regex: Regex::new(r#"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"#).unwrap(),
            secret_type: "Private Key",
            description: "Private key detected in source",
        },
        // JWT tokens
        SecretPattern {
            regex: Regex::new(r#"eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}"#).unwrap(),
            secret_type: "JWT Token",
            description: "JWT token detected in source",
        },
        // Generic passwords
        SecretPattern {
            regex: Regex::new(r#"(?i)(?:password|passwd|pwd)\s*[=:]\s*['"][^'"]{8,}['"]"#).unwrap(),
            secret_type: "Password",
            description: "Hardcoded password detected",
        },
        // Anthropic
        SecretPattern {
            regex: Regex::new(r#"sk-ant-[A-Za-z0-9\-_]{40,}"#).unwrap(),
            secret_type: "Anthropic API Key",
            description: "Anthropic API key detected",
        },
        // OpenAI
        SecretPattern {
            regex: Regex::new(r#"sk-[A-Za-z0-9]{48}"#).unwrap(),
            secret_type: "OpenAI API Key",
            description: "OpenAI API key detected",
        },
    ]
});

/// Scan a file for exposed secrets
pub fn scan(path: &Path, source: &str, language: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let file_str = path.to_string_lossy().to_string();

    // Skip common false-positive files
    let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
    if matches!(filename, ".env.example" | ".env.sample" | "example.env") {
        return findings;
    }

    for (line_num, line) in source.lines().enumerate() {
        // Skip comments that are clearly documentation
        let trimmed = line.trim();
        if trimmed.starts_with("//") && trimmed.contains("example") {
            continue;
        }

        for pattern in PATTERNS.iter() {
            if pattern.regex.is_match(line) {
                let mut metadata = HashMap::new();
                metadata.insert("secret_type".into(), pattern.secret_type.into());

                findings.push(Finding {
                    category: "secret".into(),
                    severity: "critical".into(),
                    file: file_str.clone(),
                    line: line_num + 1,
                    title: format!("{} exposed", pattern.secret_type),
                    description: pattern.description.into(),
                    snippet: redact_secret(line.trim()),
                    language: language.into(),
                    metadata,
                });
            }
        }
    }

    findings
}

/// Redact the actual secret value in the snippet for safe display
fn redact_secret(line: &str) -> String {
    // Replace long alphanumeric sequences with redaction
    let re = Regex::new(r#"[A-Za-z0-9\-_/+=]{20,}"#).unwrap();
    re.replace_all(line, "***REDACTED***").to_string()
}
