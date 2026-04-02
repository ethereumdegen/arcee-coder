use crate::api::types::Usage;

/// Tracks cumulative cost across a session.
#[derive(Debug, Clone, Default)]
pub struct CostTracker {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub turns: u32,
}

impl CostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_usage(&mut self, usage: &Usage) {
        self.total_input_tokens += usage.input_tokens;
        self.total_output_tokens += usage.output_tokens;
        self.total_cache_creation_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
        self.total_cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
        self.turns += 1;
    }

    /// Estimate cost in USD based on the model.
    pub fn estimate_cost_usd(&self, model: &str) -> f64 {
        let (input_rate, output_rate, cache_write_rate, cache_read_rate) = model_rates(model);

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
}

/// Returns (input, output, cache_write, cache_read) rates per 1M tokens.
/// Arcee pricing — rates are approximate; check https://arcee.ai for current pricing.
fn model_rates(model: &str) -> (f64, f64, f64, f64) {
    if model.contains("large") {
        // Trinity Large / Trinity Large Thinking
        (3.0, 15.0, 0.0, 0.0)
    } else if model.contains("mini") {
        // Trinity Mini
        (0.50, 2.50, 0.0, 0.0)
    } else {
        // Default estimate
        (3.0, 15.0, 0.0, 0.0)
    }
}
