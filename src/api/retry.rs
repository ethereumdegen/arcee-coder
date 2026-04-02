use crate::api::errors::ApiError;
use std::time::Duration;
use tokio::time::sleep;

const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 1000;
const MAX_DELAY_MS: u64 = 30000;

pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: MAX_RETRIES,
            base_delay: Duration::from_millis(BASE_DELAY_MS),
            max_delay: Duration::from_millis(MAX_DELAY_MS),
        }
    }
}

pub async fn with_retry<F, Fut, T>(config: &RetryConfig, mut f: F) -> Result<T, ApiError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, ApiError>>,
{
    let mut attempt = 0;

    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;

                if attempt >= config.max_retries || !e.is_retryable() {
                    return Err(e);
                }

                let delay = match &e {
                    ApiError::RateLimit {
                        retry_after_secs: Some(secs),
                    } => Duration::from_secs(*secs),
                    _ => {
                        let backoff = config.base_delay.as_millis() as u64 * 2u64.pow(attempt - 1);
                        // Add jitter: 50-150% of backoff
                        let jitter_factor = 0.5 + rand_factor();
                        let delay_ms =
                            (backoff as f64 * jitter_factor) as u64;
                        Duration::from_millis(delay_ms.min(config.max_delay.as_millis() as u64))
                    }
                };

                let category = e.category();
                eprintln!(
                    "Request failed ({category:?}), retrying in {:.1}s (attempt {attempt}/{})...",
                    delay.as_secs_f64(),
                    config.max_retries
                );

                sleep(delay).await;
            }
        }
    }
}

/// Simple pseudo-random factor between 0.0 and 1.0 using time-based seed.
fn rand_factor() -> f64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 1000) as f64 / 1000.0
}
