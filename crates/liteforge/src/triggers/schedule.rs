//! Schedule-based trigger.

use super::{Trigger, TriggerError, TriggerEvent, TriggerStatus};
use crate::scheduler::{CronSchedule, IntervalSchedule, OnceSchedule, Schedule};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A trigger that fires based on a schedule.
#[derive(Debug, Clone)]
pub struct ScheduleTrigger {
    id: String,
    schedule: Arc<Mutex<ScheduleKind>>,
    status: Arc<Mutex<TriggerStatus>>,
    fire_count: Arc<Mutex<u64>>,
}

/// Internal schedule kind wrapper.
#[derive(Debug, Clone)]
enum ScheduleKind {
    Once(OnceSchedule),
    Interval(IntervalSchedule),
    Cron(CronSchedule),
}

impl ScheduleKind {
    fn should_run(&self) -> bool {
        match self {
            ScheduleKind::Once(s) => s.should_run(),
            ScheduleKind::Interval(s) => s.should_run(),
            ScheduleKind::Cron(s) => s.should_run(),
        }
    }

    fn advance(&mut self) {
        match self {
            ScheduleKind::Once(s) => s.advance(),
            ScheduleKind::Interval(s) => s.advance(),
            ScheduleKind::Cron(s) => s.advance(),
        }
    }

    fn is_exhausted(&self) -> bool {
        match self {
            ScheduleKind::Once(s) => s.is_exhausted(),
            ScheduleKind::Interval(s) => s.is_exhausted(),
            ScheduleKind::Cron(s) => s.is_exhausted(),
        }
    }

    fn next_run(&self) -> Option<u64> {
        match self {
            ScheduleKind::Once(s) => s.next_run(),
            ScheduleKind::Interval(s) => s.next_run(),
            ScheduleKind::Cron(s) => s.next_run(),
        }
    }
}

impl ScheduleTrigger {
    /// Create a trigger that fires once immediately.
    pub fn once(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            schedule: Arc::new(Mutex::new(ScheduleKind::Once(OnceSchedule::now()))),
            status: Arc::new(Mutex::new(TriggerStatus::Stopped)),
            fire_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a trigger that fires once after a delay.
    pub fn once_after(id: impl Into<String>, delay: Duration) -> Self {
        Self {
            id: id.into(),
            schedule: Arc::new(Mutex::new(ScheduleKind::Once(OnceSchedule::after(delay)))),
            status: Arc::new(Mutex::new(TriggerStatus::Stopped)),
            fire_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a trigger that fires at fixed intervals.
    pub fn interval(id: impl Into<String>, interval: Duration) -> Self {
        Self {
            id: id.into(),
            schedule: Arc::new(Mutex::new(ScheduleKind::Interval(IntervalSchedule::new(
                interval,
            )))),
            status: Arc::new(Mutex::new(TriggerStatus::Stopped)),
            fire_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a trigger with an interval in seconds.
    pub fn every_secs(id: impl Into<String>, secs: u64) -> Self {
        Self::interval(id, Duration::from_secs(secs))
    }

    /// Create a trigger with an interval in minutes.
    pub fn every_mins(id: impl Into<String>, mins: u64) -> Self {
        Self::interval(id, Duration::from_secs(mins * 60))
    }

    /// Create a trigger with an interval in hours.
    pub fn every_hours(id: impl Into<String>, hours: u64) -> Self {
        Self::interval(id, Duration::from_secs(hours * 3600))
    }

    /// Create a trigger based on a cron expression.
    pub fn cron(id: impl Into<String>, expression: &str) -> Result<Self, TriggerError> {
        let cron = CronSchedule::new(expression);
        if !cron.is_valid() {
            return Err(TriggerError::config(format!(
                "Invalid cron expression: {}",
                expression
            )));
        }

        Ok(Self {
            id: id.into(),
            schedule: Arc::new(Mutex::new(ScheduleKind::Cron(cron))),
            status: Arc::new(Mutex::new(TriggerStatus::Stopped)),
            fire_count: Arc::new(Mutex::new(0)),
        })
    }

    /// Get the number of times this trigger has fired.
    pub fn fire_count(&self) -> u64 {
        *self.fire_count.lock().unwrap()
    }

    /// Get the next scheduled run time.
    pub fn next_run(&self) -> Option<u64> {
        self.schedule.lock().unwrap().next_run()
    }

    /// Check if the schedule is exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.schedule.lock().unwrap().is_exhausted()
    }

    /// Set max runs for interval schedules.
    pub fn with_max_runs(self, max: u64) -> Self {
        let mut schedule = self.schedule.lock().unwrap();
        if let ScheduleKind::Interval(ref mut s) = *schedule {
            *s = s.clone().with_max_runs(max);
        }
        drop(schedule);
        self
    }
}

impl Trigger for ScheduleTrigger {
    fn id(&self) -> &str {
        &self.id
    }

    fn trigger_type(&self) -> &str {
        "schedule"
    }

    fn status(&self) -> TriggerStatus {
        *self.status.lock().unwrap()
    }

    fn start(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        if *status == TriggerStatus::Running {
            return Err(TriggerError::already_running());
        }
        *status = TriggerStatus::Running;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        *status = TriggerStatus::Stopped;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        if *status != TriggerStatus::Running {
            return Err(TriggerError::not_running());
        }
        *status = TriggerStatus::Paused;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        if *status != TriggerStatus::Paused {
            return Err(TriggerError::runtime("Trigger is not paused"));
        }
        *status = TriggerStatus::Running;
        Ok(())
    }

    fn poll(&self) -> Option<TriggerEvent> {
        let status = *self.status.lock().unwrap();
        if status != TriggerStatus::Running {
            return None;
        }

        let mut schedule = self.schedule.lock().unwrap();
        if schedule.should_run() {
            schedule.advance();

            let mut count = self.fire_count.lock().unwrap();
            *count += 1;
            let fire_num = *count;

            Some(TriggerEvent::new(
                &self.id,
                "schedule",
                serde_json::json!({
                    "fire_count": fire_num,
                    "exhausted": schedule.is_exhausted(),
                }),
            ))
        } else {
            None
        }
    }

    fn has_pending(&self) -> bool {
        let status = *self.status.lock().unwrap();
        if status != TriggerStatus::Running {
            return false;
        }
        self.schedule.lock().unwrap().should_run()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_trigger_once() {
        let mut trigger = ScheduleTrigger::once("test");
        trigger.start().unwrap();

        let _event = trigger.poll().unwrap();

        // Should be exhausted after one fire
        assert!(trigger.is_exhausted());
        assert!(trigger.poll().is_none());
    }

    #[test]
    fn test_schedule_trigger_interval() {
        let mut trigger = ScheduleTrigger::interval("test", Duration::from_millis(10));
        trigger.start().unwrap();

        // First poll should fire immediately
        let _event = trigger.poll().unwrap();
        assert_eq!(trigger.fire_count(), 1);

        // Second poll should not fire yet
        assert!(trigger.poll().is_none());

        // Wait and try again
        std::thread::sleep(Duration::from_millis(15));
        let _event = trigger.poll().unwrap();
        assert_eq!(trigger.fire_count(), 2);
    }

    #[test]
    fn test_schedule_trigger_with_max_runs() {
        let mut trigger =
            ScheduleTrigger::interval("test", Duration::from_millis(1)).with_max_runs(2);
        trigger.start().unwrap();

        trigger.poll().unwrap(); // Fire 1
        std::thread::sleep(Duration::from_millis(5));
        trigger.poll().unwrap(); // Fire 2

        assert!(trigger.is_exhausted());
    }

    #[test]
    fn test_schedule_trigger_not_running() {
        let trigger = ScheduleTrigger::once("test");
        assert!(trigger.poll().is_none()); // Not started
    }

    #[test]
    fn test_schedule_trigger_cron() {
        // This test just verifies cron parsing works
        let trigger = ScheduleTrigger::cron("test", "0 * * * *");
        assert!(trigger.is_ok());

        let trigger = ScheduleTrigger::cron("test", "invalid");
        assert!(trigger.is_err());
    }
}
