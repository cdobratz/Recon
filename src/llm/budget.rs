//! Estimated spend for one LLM call.
//!
//! Prices are coarse published rates used only to refuse a request before it
//! is sent. Ollama is local, so its price is zero.

use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy)]
pub struct TokenPrice {
    pub input_per_million: f64,
    pub output_per_million: f64,
}

pub fn price_for(provider: &str) -> TokenPrice {
    match provider {
        "ollama" => TokenPrice {
            input_per_million: 0.0,
            output_per_million: 0.0,
        },
        "openai" => TokenPrice {
            input_per_million: 0.15,
            output_per_million: 0.60,
        },
        "anthropic" => TokenPrice {
            input_per_million: 3.0,
            output_per_million: 15.0,
        },
        _ => TokenPrice {
            input_per_million: 10.0,
            output_per_million: 30.0,
        },
    }
}

pub fn estimate_tokens(text: &str) -> u32 {
    ((text.len() as u32) / 4).max(1)
}

pub fn estimate_cost(provider: &str, input_tokens: u32, output_tokens: u32) -> f64 {
    let price = price_for(provider);
    (input_tokens as f64) * price.input_per_million / 1_000_000.0
        + (output_tokens as f64) * price.output_per_million / 1_000_000.0
}

/// Refuse the call when the estimate would pass `max_cost_usd`.
pub fn ensure_budget(
    provider: &str,
    input: &str,
    max_output_tokens: u32,
    spent_usd: f64,
    max_cost_usd: f64,
) -> Result<f64> {
    let estimate = estimate_cost(provider, estimate_tokens(input), max_output_tokens);
    if spent_usd + estimate > max_cost_usd + 1e-9 {
        bail!(
            "estimated LLM cost ${:.4} would pass the ${:.4} cap (already spent ${:.4}). \
             Raise --max-cost or llm.max_cost_usd, or use the local ollama provider.",
            estimate,
            max_cost_usd,
            spent_usd
        );
    }
    Ok(estimate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ollama_is_free_and_hosted_calls_can_be_refused() {
        let huge = "a".repeat(400_000);
        assert!(ensure_budget("ollama", &huge, 8_000, 0.0, 0.0).is_ok());
        let err = ensure_budget("openai", &huge, 8_000, 0.0, 0.01).unwrap_err();
        assert!(err.to_string().contains("cap"), "{err}");
    }
}
