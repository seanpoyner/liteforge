//! Retry utilities for transient API failures.

use crate::error::{Result, ForgeError};
use std::future::Future;
use std::time::Duration;

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Initial delay between retries.
    pub initial_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Backoff multiplier for exponential backoff.
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config with the given max retries.
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Set the initial delay.
    pub fn initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set the maximum delay.
    pub fn max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Set the backoff multiplier.
    pub fn backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Calculate the delay for a given attempt (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return self.initial_delay;
        }

        let delay_ms =
            self.initial_delay.as_millis() as f64 * self.backoff_multiplier.powi(attempt as i32);
        let delay = Duration::from_millis(delay_ms as u64);

        std::cmp::min(delay, self.max_delay)
    }
}

/// Check if an error is retryable.
pub fn is_retryable(error: &ForgeError) -> bool {
    match error {
        // Retry on rate limits
        ForgeError::RateLimit { .. } => true,
        // Retry on server errors (5xx)
        ForgeError::Server { status_code, .. } => *status_code >= 500,
        // Retry on network errors
        ForgeError::Network { .. } => true,
        // Retry on timeouts
        ForgeError::Timeout { .. } => true,
        // Don't retry on other errors (auth, invalid request, etc.)
        _ => false,
    }
}

/// Execute a function with retry logic (synchronous).
pub fn with_retry<F, T>(config: &RetryConfig, mut f: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let mut last_error = None;

    for attempt in 0..=config.max_retries {
        match f() {
            Ok(result) => return Ok(result),
            Err(e) => {
                if !is_retryable(&e) {
                    return Err(e);
                }

                last_error = Some(e);

                if attempt < config.max_retries {
                    let delay = config.delay_for_attempt(attempt);
                    std::thread::sleep(delay);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| ForgeError::other("Retry failed with no error captured")))
}

/// Execute an async function with retry logic.
pub async fn with_retry_async<F, Fut, T>(config: &RetryConfig, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 0..=config.max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if !is_retryable(&e) {
                    return Err(e);
                }

                last_error = Some(e);

                if attempt < config.max_retries {
                    let delay = config.delay_for_attempt(attempt);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| ForgeError::other("Retry failed with no error captured")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
    }

    #[test]
    fn test_delay_for_attempt() {
        let config = RetryConfig::default();

        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(500));
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(1000));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(2000));
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(4000));
    }

    #[test]
    fn test_delay_max_cap() {
        let config = RetryConfig::default().max_delay(Duration::from_millis(1500));

        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(500));
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(1000));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(1500)); // Capped
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(1500)); // Capped
    }

    #[test]
    fn test_with_retry_success() {
        let config = RetryConfig::new(3);
        let result = with_retry(&config, || Ok::<_, ForgeError>(42));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_with_retry_eventual_success() {
        let config = RetryConfig::new(3).initial_delay(Duration::from_millis(1));
        let attempts = RefCell::new(0);

        let result = with_retry(&config, || {
            let mut count = attempts.borrow_mut();
            *count += 1;
            if *count < 3 {
                Err(ForgeError::network("transient"))
            } else {
                Ok(42)
            }
        });

        assert_eq!(result.unwrap(), 42);
        assert_eq!(*attempts.borrow(), 3);
    }

    #[test]
    fn test_with_retry_non_retryable() {
        let config = RetryConfig::new(3).initial_delay(Duration::from_millis(1));
        let attempts = RefCell::new(0);

        let result = with_retry(&config, || {
            let mut count = attempts.borrow_mut();
            *count += 1;
            Err::<i32, _>(ForgeError::InvalidRequest {
                message: "bad request".into(),
                response: None,
            })
        });

        assert!(result.is_err());
        assert_eq!(*attempts.borrow(), 1); // Should not retry
    }

    #[tokio::test]
    async fn test_with_retry_async_success() {
        let config = RetryConfig::new(3);
        let result = with_retry_async(&config, || async { Ok::<_, ForgeError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }
}
