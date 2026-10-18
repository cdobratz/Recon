//! Inline `recon:ignore <rule-id>` suppressions.
//!
//! The marker is recognized on the finding's line and on the previous line,
//! including `//`, `#`, and `/* */` comments.

use super::Finding;

pub fn apply(findings: Vec<Finding>, source: &str) -> Vec<Finding> {
    let lines: Vec<&str> = source.lines().collect();
    findings
        .into_iter()
        .filter(|finding| {
            let Some(rule_id) = finding.metadata.get("rule_id") else {
                return true;
            };
            !line_suppresses(&lines, finding.line, rule_id)
        })
        .collect()
}

fn line_suppresses(lines: &[&str], line: usize, rule_id: &str) -> bool {
    if line == 0 {
        return false;
    }
    let same = lines.get(line - 1).copied().unwrap_or("");
    let previous = if line >= 2 {
        lines.get(line - 2).copied().unwrap_or("")
    } else {
        ""
    };
    ignored_rules(same).contains(&rule_id) || ignored_rules(previous).contains(&rule_id)
}

fn ignored_rules(line: &str) -> Vec<&str> {
    let Some(index) = line.find("recon:ignore") else {
        return Vec::new();
    };
    let rest = &line[index + "recon:ignore".len()..];
    let rest = rest.split("*/").next().unwrap_or(rest);
    rest.split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter(|token| !token.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn finding(line: usize, rule: &str) -> Finding {
        let mut metadata = HashMap::new();
        metadata.insert("rule_id".into(), rule.into());
        Finding {
            category: "agent".into(),
            severity: "high".into(),
            file: "main.go".into(),
            line,
            title: "shell".into(),
            description: "shell".into(),
            snippet: "exec.Command(\"az\")".into(),
            language: "Go".into(),
            metadata,
        }
    }

    #[test]
    fn same_line_and_previous_line_comments() {
        let source = "\
# recon:ignore recon/shell-execution
os.system(\"ls\")
exec(\"ls\")
execSync(\"ls\"); // recon:ignore recon/shell-execution
/* recon:ignore recon/shell-execution, recon/agent-cloud-spend */
exec.Command(\"az\")
";
        let findings = vec![
            finding(2, "recon/shell-execution"),
            finding(3, "recon/shell-execution"),
            finding(4, "recon/shell-execution"),
            finding(6, "recon/shell-execution"),
            finding(6, "recon/agent-cloud-spend"),
        ];
        let kept = apply(findings, source);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].line, 3);
    }
}
