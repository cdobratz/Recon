//! LLM context built from findings only.
//!
//! The text never includes a source file. Each snippet is whitespace-collapsed
//! and cut to `max_snippet_chars`.

use crate::scanner::Finding;

pub fn findings_context(
    findings: &[Finding],
    max_findings: usize,
    max_snippet_chars: usize,
) -> String {
    let mut lines = Vec::new();
    for finding in findings.iter().take(max_findings) {
        let rule = finding
            .metadata
            .get("rule_id")
            .map(|id| id.as_str())
            .unwrap_or(finding.category.as_str());
        let snippet = truncate(&normalize(&finding.snippet), max_snippet_chars);
        lines.push(format!(
            "- [{}] {} {}:{} {} | {}",
            finding.severity, rule, finding.file, finding.line, finding.title, snippet
        ));
    }
    if findings.len() > max_findings {
        lines.push(format!(
            "- … {} more finding(s) omitted by llm.max_findings",
            findings.len() - max_findings
        ));
    }
    lines.join("\n")
}

pub fn for_category<'a>(findings: &'a [Finding], category: &str) -> Vec<&'a Finding> {
    findings
        .iter()
        .filter(|finding| category_matches(category, finding))
        .collect()
}

fn category_matches(category: &str, finding: &Finding) -> bool {
    match category {
        "security" => matches!(
            finding.category.as_str(),
            "secret" | "db_connection" | "agent"
        ),
        "api_efficiency" => matches!(finding.category.as_str(), "api_endpoint" | "function"),
        "agent_data" => matches!(finding.category.as_str(), "agent" | "mcp" | "skill"),
        _ => false,
    }
}

fn normalize(snippet: &str) -> String {
    snippet.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn finding(snippet: &str) -> Finding {
        let mut metadata = HashMap::new();
        metadata.insert("rule_id".into(), "recon/shell-execution".into());
        Finding {
            category: "agent".into(),
            severity: "high".into(),
            file: "main.go".into(),
            line: 4,
            title: "shell".into(),
            description: "unused description".into(),
            snippet: snippet.into(),
            language: "Go".into(),
            metadata,
        }
    }

    #[test]
    fn context_uses_snippets_and_drops_the_rest_of_a_file() {
        let file_only = "UNIQUE_FILE_BODY_SHOULD_NOT_APPEAR";
        let text = findings_context(&[finding("exec.Command(\"az\")")], 10, 80);
        assert!(text.contains("exec.Command"));
        assert!(text.contains("recon/shell-execution"));
        assert!(!text.contains(file_only));
        assert!(!text.contains("unused description"));
    }

    #[test]
    fn snippets_are_truncated() {
        let text = findings_context(&[finding(&"x".repeat(500))], 10, 40);
        assert!(!text.contains(&"x".repeat(41)));
        assert!(text.contains('…'));
    }
}
