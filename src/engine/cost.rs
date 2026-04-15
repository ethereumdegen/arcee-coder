use crate::api::types::{ModelInfo, Usage};
use std::collections::HashMap;

/// Per-model pricing: (input, output, cache_write, cache_read) rates per 1M tokens.
pub type PricingTable = HashMap<String, (f64, f64, f64, f64)>;

/// Build a pricing table from the API's model list.
/// Converts per-token string prices (e.g. "0.00000025") to per-1M-token rates.
pub fn build_pricing_table(models: &[ModelInfo]) -> PricingTable {
    let mut table = PricingTable::new();
    for model in models {
        if let Some(ref pricing) = model.pricing {
            let prompt = parse_rate(&pricing.prompt);
            let completion = parse_rate(&pricing.completion);
            let cache_read = parse_rate(&pricing.input_cache_reads);
            let cache_write = parse_rate(&pricing.input_cache_writes);
            // Only add if we have at least prompt or completion pricing
            if prompt > 0.0 || completion > 0.0 {
                table.insert(model.id.clone(), (prompt, completion, cache_write, cache_read));
            }
        }
    }
    table
}

/// Parse a per-token rate string into a per-1M-token rate.
fn parse_rate(s: &Option<String>) -> f64 {
    s.as_ref()
        .and_then(|v| v.parse::<f64>().ok())
        .map(|per_token| per_token * 1_000_000.0)
        .unwrap_or(0.0)
}

/// Tracks cumulative cost across a session.
/// Per-model token accumulation: (input_tokens, output_tokens, cache_write_tokens, cache_read_tokens).
type PerModelUsage = HashMap<String, (u64, u64, u64, u64)>;

#[derive(Debug, Clone, Default)]
pub struct CostTracker {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub turns: u32,
    /// The input token count from the most recent API response.
    /// Used for accurate auto-compact threshold checks (instead of char-based estimates).
    pub last_input_tokens: u64,
    /// Per-model usage breakdown for accurate cost calculation across mixed models.
    per_model_usage: PerModelUsage,
    pricing_table: PricingTable,
}

impl CostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pricing(table: PricingTable) -> Self {
        Self {
            pricing_table: table,
            ..Self::default()
        }
    }

    pub fn add_usage(&mut self, usage: &Usage) {
        self.total_input_tokens += usage.input_tokens;
        self.total_output_tokens += usage.output_tokens;
        self.total_cache_creation_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
        self.total_cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
        self.last_input_tokens = usage.input_tokens;
        self.turns += 1;
    }

    /// Record usage for a specific model (for accurate per-model cost calculation).
    pub fn add_usage_for_model(&mut self, model: &str, usage: &Usage) {
        self.add_usage(usage);
        let entry = self.per_model_usage.entry(model.to_string()).or_insert((0, 0, 0, 0));
        entry.0 += usage.input_tokens;
        entry.1 += usage.output_tokens;
        entry.2 += usage.cache_creation_input_tokens.unwrap_or(0);
        entry.3 += usage.cache_read_input_tokens.unwrap_or(0);
    }

    /// Calculate total cost across all models using per-model rates.
    pub fn total_cost_usd(&self) -> f64 {
        if self.per_model_usage.is_empty() {
            // Fallback: no per-model data, use aggregate with default rates
            return self.estimate_cost_usd("trinity-large-thinking");
        }
        self.per_model_usage
            .iter()
            .map(|(model, (inp, out, cw, cr))| {
                let (ir, or, cwr, crr) = self.model_rates(model);
                *inp as f64 * ir / 1_000_000.0
                    + *out as f64 * or / 1_000_000.0
                    + *cw as f64 * cwr / 1_000_000.0
                    + *cr as f64 * crr / 1_000_000.0
            })
            .sum()
    }

    /// Estimate cost in USD based on the model.
    pub fn estimate_cost_usd(&self, model: &str) -> f64 {
        let (input_rate, output_rate, cache_write_rate, cache_read_rate) = self.model_rates(model);

        let input_cost = self.total_input_tokens as f64 * input_rate / 1_000_000.0;
        let output_cost = self.total_output_tokens as f64 * output_rate / 1_000_000.0;
        let cache_write_cost =
            self.total_cache_creation_tokens as f64 * cache_write_rate / 1_000_000.0;
        let cache_read_cost = self.total_cache_read_tokens as f64 * cache_read_rate / 1_000_000.0;

        input_cost + output_cost + cache_write_cost + cache_read_cost
    }

    pub fn summary(&self, model: &str) -> String {
        let cost = self.estimate_cost_usd(model);
        format!(
            "Tokens: {}in / {}out | Cache: {}write / {}read | Turns: {} | Cost: ${:.4}",
            self.total_input_tokens,
            self.total_output_tokens,
            self.total_cache_creation_tokens,
            self.total_cache_read_tokens,
            self.turns,
            cost
        )
    }

    /// Look up rates from the pricing table, falling back to hardcoded defaults.
    fn model_rates(&self, model: &str) -> (f64, f64, f64, f64) {
        // Exact match first
        if let Some(rates) = self.pricing_table.get(model) {
            return *rates;
        }
        // Hardcoded fallback
        hardcoded_rates(model)
    }
}

/// Hardcoded fallback rates per 1M tokens (used when API pricing unavailable).
fn hardcoded_rates(model: &str) -> (f64, f64, f64, f64) {
    if model.contains("large") {
        // Trinity Large / Trinity Large Thinking — approximate OpenRouter rates
        (0.25, 0.90, 0.0, 0.0)
    } else if model.contains("mini") {
        // Trinity Mini — free/very cheap
        (0.02, 0.07, 0.0, 0.0)
    } else {
        // Default estimate
        (0.25, 0.90, 0.0, 0.0)
    }
}
