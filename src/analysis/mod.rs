pub mod security;
pub mod api_efficiency;
pub mod agent_data;

use crate::config::ReconConfig;
use crate::scanner::ScanResults;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Result of AI-powered analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResults {
    pub security: Option<AnalysisReport>,
    pub api_efficiency: Option<AnalysisReport>,
    pub agent_data: Option<AnalysisReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub category: String,
    pub summary: String,
    pub risk_score: u8, // 0-100
    pub recommendations: Vec<String>,
    pub details: String,
}

/// Run AI-powered analysis on scan results
pub async fn run_analysis(
    scan_results: &ScanResults,
    provider: &str,
    model: Option<&str>,
    categories: Option<&[String]>,
    config: &ReconConfig,
) -> Result<AnalysisResults> {
    let cats: Vec<&str> = categories
        .map(|c| c.iter().map(|s| s.as_str()).collect())
        .unwrap_or_else(|| vec!["security", "api_efficiency", "agent_data"]);

    let mut results = AnalysisResults {
        security: None,
        api_efficiency: None,
        agent_data: None,
    };

    // For now, run rule-based analysis (LLM integration in Phase 2)
    if cats.contains(&"security") {
        results.security = Some(security::analyze(scan_results));
    }
    if cats.contains(&"api_efficiency") {
        results.api_efficiency = Some(api_efficiency::analyze(scan_results));
    }
    if cats.contains(&"agent_data") {
        results.agent_data = Some(agent_data::analyze(scan_results));
    }

    Ok(results)
}
