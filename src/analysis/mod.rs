pub mod agent_data;
pub mod api_efficiency;
pub mod llm;
pub mod security;

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
    /// Model narrative. Present only after `--llm`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_summary: Option<String>,
    /// Estimated spend of the LLM call that produced `llm_summary`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_cost_usd: Option<f64>,
}

impl AnalysisReport {
    pub fn new(
        category: impl Into<String>,
        summary: impl Into<String>,
        risk_score: u8,
        recommendations: Vec<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            category: category.into(),
            summary: summary.into(),
            risk_score,
            recommendations,
            details: details.into(),
            llm_summary: None,
            estimated_cost_usd: None,
        }
    }
}

/// Options for one analysis run. `use_llm` is the only path that contacts a model.
pub struct AnalysisRequest<'a> {
    pub provider: &'a str,
    pub model: Option<&'a str>,
    pub categories: Option<&'a [String]>,
    pub use_llm: bool,
    pub max_cost_usd: Option<f64>,
}

/// Score a finished scan. Rule scores always run. The LLM runs only when
/// `request.use_llm` is set, and it receives findings plus short snippets.
pub async fn run_analysis(
    scan_results: &ScanResults,
    config: &ReconConfig,
    request: AnalysisRequest<'_>,
) -> Result<AnalysisResults> {
    let cats: Vec<&str> = request
        .categories
        .map(|c| c.iter().map(|s| s.as_str()).collect())
        .unwrap_or_else(|| vec!["security", "api_efficiency", "agent_data"]);

    let mut results = AnalysisResults {
        security: None,
        api_efficiency: None,
        agent_data: None,
    };

    if cats.contains(&"security") {
        results.security = Some(security::analyze(scan_results));
    }
    if cats.contains(&"api_efficiency") {
        results.api_efficiency = Some(api_efficiency::analyze(scan_results));
    }
    if cats.contains(&"agent_data") {
        results.agent_data = Some(agent_data::analyze(scan_results));
    }

    if request.use_llm {
        let pass = llm::LlmPass {
            provider_name: request.provider,
            model: request.model,
            config,
            max_cost_usd: request.max_cost_usd.unwrap_or(config.llm.max_cost_usd),
        };
        let mut spent = 0.0;
        if let Some(report) = results.security.as_mut() {
            llm::annotate(report, &scan_results.findings, &pass, &mut spent).await?;
        }
        if let Some(report) = results.api_efficiency.as_mut() {
            llm::annotate(report, &scan_results.findings, &pass, &mut spent).await?;
        }
        if let Some(report) = results.agent_data.as_mut() {
            llm::annotate(report, &scan_results.findings, &pass, &mut spent).await?;
        }
    }

    Ok(results)
}
