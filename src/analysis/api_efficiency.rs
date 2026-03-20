use super::AnalysisReport;
use crate::scanner::ScanResults;

/// Analyze scan results for API efficiency issues
pub fn analyze(results: &ScanResults) -> AnalysisReport {
    let mut risk_score: u32 = 0;
    let mut recommendations = Vec::new();

    let endpoints = results.findings.iter().filter(|f| f.category == "api_endpoint").count();
    let functions = results.findings.iter().filter(|f| f.category == "function").count();

    // Flag repos with many endpoints but no pagination patterns
    if endpoints > 10 {
        risk_score += 15;
        recommendations.push(format!(
            "Found {} API endpoints. Ensure pagination is implemented for list endpoints.", endpoints
        ));
    }

    // Flag high-parameter functions as potential API bloat
    let high_param_fns = results.findings.iter()
        .filter(|f| f.category == "function" && f.severity == "medium")
        .count();
    if high_param_fns > 0 {
        risk_score += (high_param_fns as u32 * 5).min(20);
        recommendations.push(format!(
            "Found {} function(s) with 7+ parameters. Consider using request objects or breaking up.", high_param_fns
        ));
    }

    if recommendations.is_empty() {
        recommendations.push("No significant API efficiency issues detected.".into());
    }

    let summary = match risk_score.min(100) {
        0..=20 => "Good API efficiency",
        21..=50 => "Minor API efficiency concerns",
        51..=80 => "Notable API efficiency issues",
        _ => "Significant API efficiency problems",
    };

    AnalysisReport {
        category: "api_efficiency".into(),
        summary: summary.into(),
        risk_score: risk_score.min(100) as u8,
        recommendations,
        details: format!(
            "Found {} endpoints and {} functions across {} files.",
            endpoints, functions, results.files_scanned
        ),
    }
}
