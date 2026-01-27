use anyhow::Result;
use std::future::Future;
use tokio::time::{sleep, Duration};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_base: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            exponential_base: 2,
        }
    }
}

/// Retry an async operation with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    mut operation: F,
    config: &RetryConfig,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay_ms;

    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt >= config.max_attempts => {
                anyhow::bail!(
                    "Operation failed after {} attempts. Last error: {}",
                    attempt,
                    e
                );
            }
            Err(e) => {
                eprintln!(
                    "Attempt {}/{} failed: {}. Retrying in {}ms...",
                    attempt, config.max_attempts, e, delay
                );

                sleep(Duration::from_millis(delay)).await;

                // Exponential backoff with cap
                delay = (delay * config.exponential_base as u64).min(config.max_delay_ms);
            }
        }
    }
}

/// Retry with default configuration
pub async fn retry<F, Fut, T, E>(operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    retry_with_backoff(operation, &RetryConfig::default()).await
}

/// Wrap an async operation with timeout
pub async fn with_timeout<F, T>(
    operation: F,
    timeout_ms: u64,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    match tokio::time::timeout(Duration::from_millis(timeout_ms), operation).await {
        Ok(result) => result,
        Err(_) => anyhow::bail!("Operation timed out after {}ms", timeout_ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_succeeds_first_try() {
        let result = retry(|| async { Ok::<_, &str>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            exponential_base: 2,
        };

        let result = retry_with_backoff(
            || {
                let c = counter_clone.clone();
                async move {
                    let count = c.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err("not yet")
                    } else {
                        Ok(42)
                    }
                }
            },
            &config,
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
