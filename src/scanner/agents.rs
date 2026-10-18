use super::execution::{classify_execution, ExecutionClass};
use super::rules;
use super::Finding;
use crate::parsers::{AstHitKind, ParsedFile};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

struct AgentPattern {
    regex: Regex,
    agent_type: &'static str,
    severity: &'static str,
    description: &'static str,
}

static PATTERNS: LazyLock<Vec<AgentPattern>> = LazyLock::new(|| {
    vec![
        // LangChain
        AgentPattern {
            regex: Regex::new(r#"(?i)(?:from\s+langchain|import\s+langchain|AgentExecutor|create_react_agent|LLMChain)"#).unwrap(),
            agent_type: "LangChain",
            severity: "info",
            description: "LangChain agent framework usage",
        },
        // AutoGen
        AgentPattern {
            regex: Regex::new(r#"(?i)(?:from\s+autogen|import\s+autogen|AssistantAgent|UserProxyAgent|GroupChat)"#).unwrap(),
            agent_type: "AutoGen",
            severity: "info",
            description: "AutoGen multi-agent framework usage",
        },
        // CrewAI
        AgentPattern {
            regex: Regex::new(r#"(?i)(?:from\s+crewai|import\s+crewai|Agent\s*\(|Crew\s*\(|Task\s*\(.*?agent)"#).unwrap(),
            agent_type: "CrewAI",
            severity: "info",
            description: "CrewAI agent framework usage",
        },
        // MCP server/client config
        AgentPattern {
            regex: Regex::new(r#"(?i)(?:mcp_server|mcpServers|mcp\.json|ModelContextProtocol|ServerHandler|ClientHandler)"#).unwrap(),
            agent_type: "MCP",
            severity: "info",
            description: "Model Context Protocol configuration",
        },
        // OpenAI function calling / assistants
        AgentPattern {
            regex: Regex::new(r#"(?i)(?:function_call|tool_choice|assistants\.create|threads\.create|runs\.create)"#).unwrap(),
            agent_type: "OpenAI Assistants",
            severity: "info",
            description: "OpenAI Assistants/function calling API",
        },
        // Agent with unrestricted data access
        AgentPattern {
            regex: Regex::new(r#"(?i)(?:allow_dangerous|unrestricted|all_permissions|admin_access|root_access)"#).unwrap(),
            agent_type: "Permissions",
            severity: "critical",
            description: "Agent with unrestricted/dangerous permissions",
        },
    ]
});

/// Scan a file for agent configurations.
///
/// Shell-versus-database classification uses the tree-sitter call target when
/// the file parsed. Lines the query does not classify (imports, other
/// languages) stay on the regex fallback.
pub fn scan(
    path: &Path,
    source: &str,
    language: &str,
    parsed: Option<&ParsedFile>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let file_str = path.to_string_lossy().to_string();

    for (line_num, line) in source.lines().enumerate() {
        for pattern in PATTERNS.iter() {
            if pattern.regex.is_match(line) {
                let mut metadata = HashMap::new();
                metadata.insert("agent_type".into(), pattern.agent_type.into());

                findings.push(Finding {
                    category: "agent".into(),
                    severity: pattern.severity.into(),
                    file: file_str.clone(),
                    line: line_num + 1,
                    title: format!("{} agent detected", pattern.agent_type),
                    description: pattern.description.into(),
                    snippet: line.trim().to_string(),
                    language: language.into(),
                    metadata,
                });
            }
        }
    }

    let mut classified = HashSet::new();
    if let Some(parsed) = parsed.filter(|parsed| parsed.parsed) {
        let mut shell_lines = HashSet::new();
        for hit in parsed
            .hits
            .iter()
            .filter(|hit| hit.kind == AstHitKind::ShellCall)
        {
            if shell_lines.insert(hit.line) {
                findings.push(shell_finding(
                    &file_str,
                    language,
                    hit.line,
                    &hit.snippet,
                    "ast",
                ));
                classified.insert(hit.line);
            }
        }
        for hit in parsed
            .hits
            .iter()
            .filter(|hit| hit.kind == AstHitKind::DatabaseCall)
        {
            classified.insert(hit.line);
        }
    }

    for (line_num, line) in source.lines().enumerate() {
        let line_no = line_num + 1;
        if classified.contains(&line_no) {
            continue;
        }
        if classify_execution(line) == ExecutionClass::Shell {
            findings.push(shell_finding(
                &file_str,
                language,
                line_no,
                line.trim(),
                "regex",
            ));
        }
    }

    findings
}

fn shell_finding(
    file: &str,
    language: &str,
    line: usize,
    snippet: &str,
    detector: &str,
) -> Finding {
    let rule = rules::by_id(rules::SHELL_EXECUTION).expect("shell rule");
    let mut metadata = HashMap::new();
    metadata.insert("agent_type".into(), "Code Execution".into());
    metadata.insert("rule_id".into(), rule.id.into());
    metadata.insert("help".into(), rule.help.into());
    metadata.insert("asi".into(), rule.asi.join(","));
    metadata.insert("detector".into(), detector.into());
    Finding {
        category: "agent".into(),
        severity: rule.severity.into(),
        file: file.into(),
        line,
        title: rule.short_description.into(),
        description: rule.help.into(),
        snippet: snippet.into(),
        language: language.into(),
        metadata,
    }
}
