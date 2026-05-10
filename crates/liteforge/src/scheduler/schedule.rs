//! Schedule types for defining when jobs run.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Type of schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleType {
    /// Run once at a specific time.
    Once,
    /// Run at fixed intervals.
    Interval,
    /// Run based on cron expression.
    Cron,
}

/// Trait for schedules.
pub trait Schedule: Send + Sync {
    /// Get the schedule type.
    fn schedule_type(&self) -> ScheduleType;

    /// Get the next run time (Unix timestamp in milliseconds).
    fn next_run(&self) -> Option<u64>;

    /// Check if the schedule should run now.
    fn should_run(&self) -> bool {
        if let Some(next) = self.next_run() {
            let now = now_millis();
            now >= next
        } else {
            false
        }
    }

    /// Advance to the next scheduled time.
    fn advance(&mut self);

    /// Check if the schedule is exhausted (no more runs).
    fn is_exhausted(&self) -> bool {
        self.next_run().is_none()
    }
}

/// One-time schedule that runs at a specific time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnceSchedule {
    /// Target time (Unix timestamp in milliseconds).
    run_at: u64,
    /// Whether it has already run.
    exhausted: bool,
}

impl OnceSchedule {
    /// Create a schedule that runs at a specific timestamp.
    pub fn at(timestamp_ms: u64) -> Self {
        Self {
            run_at: timestamp_ms,
            exhausted: false,
        }
    }

    /// Create a schedule that runs after a delay.
    pub fn after(delay: Duration) -> Self {
        let run_at = now_millis() + delay.as_millis() as u64;
        Self {
            run_at,
            exhausted: false,
        }
    }

    /// Create a schedule that runs immediately.
    pub fn now() -> Self {
        Self {
            run_at: now_millis(),
            exhausted: false,
        }
    }
}

impl Schedule for OnceSchedule {
    fn schedule_type(&self) -> ScheduleType {
        ScheduleType::Once
    }

    fn next_run(&self) -> Option<u64> {
        if self.exhausted {
            None
        } else {
            Some(self.run_at)
        }
    }

    fn advance(&mut self) {
        self.exhausted = true;
    }
}

/// Interval-based schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalSchedule {
    /// Interval duration in milliseconds.
    interval_ms: u64,
    /// Next scheduled run time.
    next_run_at: u64,
    /// Maximum number of runs (None = unlimited).
    max_runs: Option<u64>,
    /// Number of times run so far.
    run_count: u64,
}

impl IntervalSchedule {
    /// Create a new interval schedule.
    pub fn new(interval: Duration) -> Self {
        let now = now_millis();
        Self {
            interval_ms: interval.as_millis() as u64,
            next_run_at: now,
            max_runs: None,
            run_count: 0,
        }
    }

    /// Create from seconds.
    pub fn from_secs(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }

    /// Create from minutes.
    pub fn from_mins(mins: u64) -> Self {
        Self::new(Duration::from_secs(mins * 60))
    }

    /// Create from hours.
    pub fn from_hours(hours: u64) -> Self {
        Self::new(Duration::from_secs(hours * 3600))
    }

    /// Set a delay before the first run.
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.next_run_at = now_millis() + delay.as_millis() as u64;
        self
    }

    /// Set maximum number of runs.
    pub fn with_max_runs(mut self, max: u64) -> Self {
        self.max_runs = Some(max);
        self
    }

    /// Get the interval duration.
    pub fn interval(&self) -> Duration {
        Duration::from_millis(self.interval_ms)
    }

    /// Get the run count.
    pub fn run_count(&self) -> u64 {
        self.run_count
    }
}

impl Schedule for IntervalSchedule {
    fn schedule_type(&self) -> ScheduleType {
        ScheduleType::Interval
    }

    fn next_run(&self) -> Option<u64> {
        if let Some(max) = self.max_runs {
            if self.run_count >= max {
                return None;
            }
        }
        Some(self.next_run_at)
    }

    fn advance(&mut self) {
        self.run_count += 1;
        self.next_run_at = now_millis() + self.interval_ms;
    }
}

/// Get current time in milliseconds.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_once_schedule() {
        let mut schedule = OnceSchedule::now();
        assert_eq!(schedule.schedule_type(), ScheduleType::Once);
        assert!(schedule.should_run());

        schedule.advance();
        assert!(schedule.is_exhausted());
        assert!(!schedule.should_run());
    }

    #[test]
    fn test_once_schedule_after() {
        let schedule = OnceSchedule::after(Duration::from_secs(60));
        assert!(!schedule.should_run()); // Not yet
        assert!(!schedule.is_exhausted());
    }

    #[test]
    fn test_interval_schedule() {
        let mut schedule = IntervalSchedule::new(Duration::from_millis(100));
        assert_eq!(schedule.schedule_type(), ScheduleType::Interval);
        assert!(schedule.should_run());

        schedule.advance();
        assert_eq!(schedule.run_count(), 1);
        // Next run should be in ~100ms
        assert!(!schedule.should_run());
    }

    #[test]
    fn test_interval_with_max_runs() {
        let mut schedule = IntervalSchedule::new(Duration::from_millis(1)).with_max_runs(2);

        schedule.advance();
        assert_eq!(schedule.run_count(), 1);
        assert!(!schedule.is_exhausted());

        schedule.advance();
        assert_eq!(schedule.run_count(), 2);
        assert!(schedule.is_exhausted());
    }

    #[test]
    fn test_interval_from_mins() {
        let schedule = IntervalSchedule::from_mins(5);
        assert_eq!(schedule.interval(), Duration::from_secs(300));
    }

    #[test]
    fn test_interval_from_hours() {
        let schedule = IntervalSchedule::from_hours(1);
        assert_eq!(schedule.interval(), Duration::from_secs(3600));
    }

    #[test]
    fn test_interval_with_initial_delay() {
        let schedule = IntervalSchedule::new(Duration::from_secs(10))
            .with_initial_delay(Duration::from_secs(5));

        // Should not run immediately due to delay
        assert!(!schedule.should_run());
    }

    #[test]
    fn test_schedule_type_serialize() {
        let schedule = OnceSchedule::now();
        let json = serde_json::to_string(&schedule).unwrap();
        assert!(json.contains("run_at"));
    }
}
