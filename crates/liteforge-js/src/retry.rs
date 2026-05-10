use liteforge::RetryConfig as RustRetryConfig;

#[napi]
pub struct RetryConfig {
    inner: RustRetryConfig,
}

#[napi]
impl RetryConfig {
    #[napi(constructor)]
    pub fn new(max_retries: u32) -> Self {
        Self {
            inner: RustRetryConfig::new(max_retries),
        }
    }

    #[napi]
    pub fn initial_delay(&mut self, ms: u32) -> &Self {
        self.inner = self
            .inner
            .clone()
            .initial_delay(std::time::Duration::from_millis(ms as u64));
        self
    }

    #[napi]
    pub fn max_delay(&mut self, ms: u32) -> &Self {
        self.inner = self
            .inner
            .clone()
            .max_delay(std::time::Duration::from_millis(ms as u64));
        self
    }

    #[napi]
    pub fn backoff_multiplier(&mut self, multiplier: f64) -> &Self {
        self.inner = self.inner.clone().backoff_multiplier(multiplier);
        self
    }

    #[napi(getter)]
    pub fn get_max_retries(&self) -> u32 {
        self.inner.max_retries
    }

    #[napi]
    pub fn delay_for_attempt(&self, attempt: u32) -> u32 {
        self.inner.delay_for_attempt(attempt).as_millis() as u32
    }
}
