use liteforge::scheduler::{
    CronSchedule as RustCronSchedule, IntervalSchedule as RustIntervalSchedule,
    OnceSchedule as RustOnceSchedule, Schedule,
};

#[napi(string_enum)]
pub enum ScheduleType {
    Once,
    Interval,
    Cron,
}

#[napi(string_enum)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[napi]
pub struct OnceSchedule {
    inner: RustOnceSchedule,
}

#[napi]
impl OnceSchedule {
    #[napi(factory)]
    pub fn now() -> Self {
        Self {
            inner: RustOnceSchedule::now(),
        }
    }

    #[napi(factory)]
    pub fn after_secs(seconds: u32) -> Self {
        Self {
            inner: RustOnceSchedule::after(std::time::Duration::from_secs(seconds as u64)),
        }
    }

    #[napi]
    pub fn should_run(&self) -> bool {
        self.inner.should_run()
    }

    #[napi]
    pub fn is_exhausted(&self) -> bool {
        self.inner.is_exhausted()
    }

    #[napi]
    pub fn advance(&mut self) {
        self.inner.advance();
    }
}

#[napi]
pub struct IntervalSchedule {
    inner: RustIntervalSchedule,
}

#[napi]
impl IntervalSchedule {
    #[napi(constructor)]
    pub fn new(seconds: u32) -> Self {
        Self {
            inner: RustIntervalSchedule::new(std::time::Duration::from_secs(seconds as u64)),
        }
    }

    #[napi(factory)]
    pub fn from_secs(seconds: u32) -> Self {
        Self {
            inner: RustIntervalSchedule::from_secs(seconds as u64),
        }
    }

    #[napi(factory)]
    pub fn from_mins(minutes: u32) -> Self {
        Self {
            inner: RustIntervalSchedule::from_mins(minutes as u64),
        }
    }

    #[napi]
    pub fn should_run(&self) -> bool {
        self.inner.should_run()
    }

    #[napi]
    pub fn advance(&mut self) {
        self.inner.advance();
    }

    #[napi]
    pub fn run_count(&self) -> u32 {
        self.inner.run_count() as u32
    }
}

#[napi]
pub struct CronSchedule {
    inner: RustCronSchedule,
}

#[napi]
impl CronSchedule {
    #[napi(constructor)]
    pub fn new(expression: String) -> Self {
        Self {
            inner: RustCronSchedule::new(expression),
        }
    }

    #[napi(factory)]
    pub fn every_minute() -> Self {
        Self {
            inner: RustCronSchedule::every_minute(),
        }
    }

    #[napi(factory)]
    pub fn hourly() -> Self {
        Self {
            inner: RustCronSchedule::hourly(),
        }
    }

    #[napi(factory)]
    pub fn daily() -> Self {
        Self {
            inner: RustCronSchedule::daily(),
        }
    }

    #[napi]
    pub fn should_run(&self) -> bool {
        self.inner.should_run()
    }

    #[napi]
    pub fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    #[napi]
    pub fn advance(&mut self) {
        self.inner.advance();
    }

    #[napi(getter)]
    pub fn expression(&self) -> String {
        self.inner.expression().to_string()
    }
}
