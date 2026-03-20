use super::AnalysisReport;
use crate::scanner::ScanResults;

/// Analyze scan results for agent-data interaction risks
pub fn analyze(results: &ScanResults) -> AnalysisReport {
    let mut risk_score: u32 = 0;
    let mut recommendations = Vec::new();

    let agents = results.findings.iter().filter(|f| f.category == "agent").collect::<Vec<_>>();
    let code_exec_agents = agents.iter()
        .filter(|f| f.metadata.get("agent_type").map(|t| t.as_str()) == Some("Code Execution"))
        .count();
    let dangerous_perms = agents.iter()
        .filter(|f| f.metadata.get("agent_type").map(|t| t.as_str()) == Some("Permissions"))
        .count();
    let db_connections = results.findings.iter().filter(|f| f.category == "db_connection").count();

    // Agents with code execution near DB connections = high risk
    if code_exec_agents > 0 && db_connections > 0 {
        risk_score += 35;
        recommendations.push(
            "Agents with code execution capability detected alongside database connections. \
             Ensure agents cannot directly access production databases.".into()
        );
    }

    if dangerous_perms > 0 {
        risk_score += (dangerous_perms as u32 * 20).min(40);
        recommendations.push(format!(
            "Found {} agent(s) with unrestricted permissions. Apply least-privilege access controls.", dangerous_perms
        ));
    }

    if code_exec_agents > 0 {
        risk_score += (code_exec_agents as u32 * 10).min(25);
        recommendations.push(format!(
            "Found {} agent(s) with code execution. Sandbox execution environments and limit filesystem access.", code_exec_agents
        ));
    }

    if recommendations.is_empty() {
        if agents.is_empty() {
            recommendations.push("No AI agents detected in this repository.".into());
        } else {
            recommendations.push("Agent configurations appear to follow security best practices.".into());
        }
    }

    let summary = match risk_score.min(100) {
        0..=20 => "Low agent-data risk",
        21..=50 => "Moderate agent-data risk",
        51..=80 => "High agent-data risk",
        _ => "Critical agent-data risk",
    };

    AnalysisReport {
        category: "agent_data".into(),
        summary: summary.into(),
        risk_score: risk_score.min(100) as u8,
        recommendations,
        details: format!(
            "Found {} agents ({} with code execution, {} with dangerous permissions) and {} DB connections.",
            agents.len(), code_exec_agents, dangerous_perms, db_connections
        ),
    }
}
