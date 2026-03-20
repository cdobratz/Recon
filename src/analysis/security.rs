use super::AnalysisReport;
use crate::scanner::ScanResults;

/// Analyze scan results for security risks
pub fn analyze(results: &ScanResults) -> AnalysisReport {
    let mut risk_score: u32 = 0;
    let mut recommendations = Vec::new();

    let secrets = results.findings.iter().filter(|f| f.category == "secret").count();
    let db_cred_issues = results.findings.iter()
        .filter(|f| f.category == "db_connection" && f.severity == "high")
        .count();
    let dangerous_agents = results.findings.iter()
        .filter(|f| f.category == "agent" && (f.severity == "high" || f.severity == "critical"))
        .count();

    if secrets > 0 {
        risk_score += (secrets as u32 * 15).min(50);
        recommendations.push(format!(
            "Found {} exposed secret(s). Move all secrets to environment variables or a secrets manager.", secrets
        ));
    }

    if db_cred_issues > 0 {
        risk_score += (db_cred_issues as u32 * 10).min(30);
        recommendations.push(format!(
            "Found {} database connection(s) with embedded credentials. Use environment variables instead.", db_cred_issues
        ));
    }

    if dangerous_agents > 0 {
        risk_score += (dangerous_agents as u32 * 12).min(30);
        recommendations.push(format!(
            "Found {} agent(s) with elevated permissions. Apply principle of least privilege.", dangerous_agents
        ));
    }

    let sql_injection = results.findings.iter()
        .filter(|f| f.category == "db_connection" && f.description.contains("SQL injection"))
        .count();
    if sql_injection > 0 {
        risk_score += (sql_injection as u32 * 15).min(40);
        recommendations.push(format!(
            "Found {} potential SQL injection point(s). Use parameterized queries.", sql_injection
        ));
    }

    if recommendations.is_empty() {
        recommendations.push("No critical security issues detected.".into());
    }

    let summary = match risk_score.min(100) {
        0..=20 => "Low security risk",
        21..=50 => "Moderate security risk",
        51..=80 => "High security risk",
        _ => "Critical security risk",
    };

    AnalysisReport {
        category: "security".into(),
        summary: summary.into(),
        risk_score: risk_score.min(100) as u8,
        recommendations,
        details: format!(
            "Scanned {} files. Found {} secrets, {} DB credential issues, {} dangerous agents.",
            results.files_scanned, secrets, db_cred_issues, dangerous_agents
        ),
    }
}
