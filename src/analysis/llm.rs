//! Opt-in LLM pass over findings and short snippets.
//!
//! `recon scan` never calls this. `recon analyze` calls it only with `--llm`.

use super::AnalysisReport;
use crate::config::ReconConfig;
use crate::llm::budget::{self, estimate_cost};
use crate::llm::payload;
use crate::llm::prompts;
use crate::llm::{create_provider, LlmMessage, LlmRequest};
use crate::scanner::Finding;
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LlmJson {
    summary: String,
    risk_score: f64,
    #[serde(default)]
    recommendations: Vec<String>,
}

pub struct LlmPass<'a> {
    pub provider_name: &'a str,
    pub model: Option<&'a str>,
    pub config: &'a ReconConfig,
    pub max_cost_usd: f64,
}

pub async fn annotate(
    report: &mut AnalysisReport,
    findings: &[Finding],
    pass: &LlmPass<'_>,
    spent: &mut f64,
) -> Result<()> {
    let selected = payload::for_category(findings, &report.category);
    if selected.is_empty() {
        return Ok(());
    }
    let owned: Vec<Finding> = selected.into_iter().cloned().collect();
    let context = payload::findings_context(
        &owned,
        pass.config.llm.max_findings,
        pass.config.llm.max_snippet_chars,
    );
    let user = match report.category.as_str() {
        "api_efficiency" => prompts::api_efficiency_prompt(&context),
        "agent_data" => prompts::agent_data_prompt(&context),
        _ => prompts::security_analysis_prompt(&context),
    };
    let system = "You only see findings and short snippets, never whole files. \
         Reply with one JSON object and no markdown: \
         {\"summary\": string, \"risk_score\": number, \"recommendations\": string[]}.";
    let model = pass
        .model
        .or(pass.config.llm.model.as_deref())
        .unwrap_or_else(|| default_model(pass.provider_name))
        .to_string();
    let request = LlmRequest {
        model,
        messages: vec![
            LlmMessage {
                role: "system".into(),
                content: system.into(),
            },
            LlmMessage {
                role: "user".into(),
                content: user,
            },
        ],
        max_tokens: pass.config.llm.max_tokens,
        temperature: pass.config.llm.temperature,
    };
    let input = request
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let estimate = budget::ensure_budget(
        pass.provider_name,
        &input,
        request.max_tokens,
        *spent,
        pass.max_cost_usd,
    )?;
    *spent += estimate;

    let api_key = resolve_api_key(pass.provider_name, pass.config);
    let base_url = resolve_base_url(pass.provider_name, pass.config);
    let provider = create_provider(pass.provider_name, api_key.as_deref(), base_url.as_deref())?;
    let response = provider.chat(&request).await?;
    tracing::debug!(model = %response.model, "llm response");
    let actual = response
        .usage
        .as_ref()
        .map(|usage| {
            estimate_cost(
                pass.provider_name,
                usage.prompt_tokens,
                usage.completion_tokens,
            )
        })
        .unwrap_or(estimate);
    merge_llm(report, &response.content, actual);
    Ok(())
}

pub fn merge_llm(report: &mut AnalysisReport, raw: &str, cost_usd: f64) {
    report.estimated_cost_usd = Some(cost_usd);
    match parse_llm_json(raw) {
        Some(parsed) => {
            report.llm_summary = Some(parsed.summary);
            report.details.push_str(&format!(
                "\nLLM risk score: {}.",
                parsed.risk_score.round().clamp(0.0, 100.0) as u8
            ));
            for recommendation in parsed.recommendations {
                let recommendation = recommendation.trim();
                if !recommendation.is_empty() {
                    report
                        .recommendations
                        .push(format!("LLM: {recommendation}"));
                }
            }
        }
        None => {
            report.llm_summary = Some(truncate(raw.trim(), 500));
            report
                .details
                .push_str("\nLLM response was not JSON. The rule-based score is unchanged.");
        }
    }
}

fn parse_llm_json(raw: &str) -> Option<LlmJson> {
    let trimmed = raw.trim();
    if let Ok(parsed) = serde_json::from_str::<LlmJson>(trimmed) {
        return Some(parsed);
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str(&trimmed[start..=end]).ok()
}

fn default_model(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "claude-sonnet-4-5",
        "openai" => "gpt-4o-mini",
        _ => "llama3.2",
    }
}

fn resolve_api_key(provider: &str, config: &ReconConfig) -> Option<String> {
    if let Some(key) = config.llm.api_key.as_deref().filter(|key| !key.is_empty()) {
        return Some(key.to_string());
    }
    let var = match provider {
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        _ => return None,
    };
    std::env::var(var).ok().filter(|key| !key.is_empty())
}

fn resolve_base_url(provider: &str, config: &ReconConfig) -> Option<String> {
    if let Some(url) = config.llm.base_url.as_deref().filter(|url| !url.is_empty()) {
        return Some(url.to_string());
    }
    if provider == "ollama" {
        return std::env::var("OLLAMA_HOST")
            .ok()
            .filter(|url| !url.is_empty());
    }
    None
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

    fn report() -> AnalysisReport {
        AnalysisReport::new(
            "security",
            "Low security risk",
            10,
            vec!["Keep secrets out of the repo.".into()],
            "Scanned 1 file.".to_string(),
        )
    }

    #[test]
    fn json_note_is_attached_without_replacing_the_rule_score() {
        let mut report = report();
        merge_llm(
            &mut report,
            "Here you go:\n{\"summary\":\"Rotate the key\",\"risk_score\":80,\"recommendations\":[\"Move it to env\"]}",
            0.01,
        );
        assert_eq!(report.risk_score, 10);
        assert_eq!(report.llm_summary.as_deref(), Some("Rotate the key"));
        assert!(report
            .recommendations
            .iter()
            .any(|item| item == "LLM: Move it to env"));
        assert!(report.details.contains("LLM risk score: 80"));
    }

    #[test]
    fn prose_does_not_change_the_rule_score() {
        let mut report = report();
        merge_llm(&mut report, "not json at all", 0.0);
        assert_eq!(report.risk_score, 10);
        assert_eq!(report.recommendations.len(), 1);
        assert!(report.details.contains("not JSON"));
    }
}
