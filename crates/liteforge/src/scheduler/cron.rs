//! Cron-based scheduling.

use super::schedule::{Schedule, ScheduleType};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Cron schedule based on cron expression.
///
/// Supports standard 5-field cron format: minute hour day-of-month month day-of-week
///
/// # Examples
///
/// - `"* * * * *"` - Every minute
/// - `"0 * * * *"` - Every hour
/// - `"0 0 * * *"` - Every day at midnight
/// - `"0 0 * * 0"` - Every Sunday at midnight
/// - `"*/5 * * * *"` - Every 5 minutes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronSchedule {
    /// Cron expression.
    expression: String,
    /// Parsed cron fields.
    #[serde(skip)]
    parsed: Option<ParsedCron>,
    /// Next scheduled run (calculated).
    next_run_at: Option<u64>,
    /// Maximum runs (None = unlimited).
    max_runs: Option<u64>,
    /// Run count.
    run_count: u64,
}

/// Parsed cron expression fields.
#[derive(Debug, Clone)]
struct ParsedCron {
    minutes: Vec<u8>,
    hours: Vec<u8>,
    days_of_month: Vec<u8>,
    months: Vec<u8>,
    days_of_week: Vec<u8>,
}

impl CronSchedule {
    /// Create a new cron schedule from an expression.
    pub fn new(expression: impl Into<String>) -> Self {
        let expr = expression.into();
        let parsed = Self::parse(&expr);
        let next_run_at = parsed.as_ref().and_then(Self::calculate_next);

        Self {
            expression: expr,
            parsed,
            next_run_at,
            max_runs: None,
            run_count: 0,
        }
    }

    /// Every minute.
    pub fn every_minute() -> Self {
        Self::new("* * * * *")
    }

    /// Every hour at minute 0.
    pub fn hourly() -> Self {
        Self::new("0 * * * *")
    }

    /// Every day at midnight.
    pub fn daily() -> Self {
        Self::new("0 0 * * *")
    }

    /// Every day at a specific hour.
    pub fn daily_at(hour: u8) -> Self {
        Self::new(format!("0 {} * * *", hour.min(23)))
    }

    /// Every week on Sunday at midnight.
    pub fn weekly() -> Self {
        Self::new("0 0 * * 0")
    }

    /// Every month on the 1st at midnight.
    pub fn monthly() -> Self {
        Self::new("0 0 1 * *")
    }

    /// Set maximum runs.
    pub fn with_max_runs(mut self, max: u64) -> Self {
        self.max_runs = Some(max);
        self
    }

    /// Get the cron expression.
    pub fn expression(&self) -> &str {
        &self.expression
    }

    /// Get run count.
    pub fn run_count(&self) -> u64 {
        self.run_count
    }

    /// Check if the expression is valid.
    pub fn is_valid(&self) -> bool {
        self.parsed.is_some()
    }

    /// Parse a cron expression.
    fn parse(expr: &str) -> Option<ParsedCron> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return None;
        }

        Some(ParsedCron {
            minutes: Self::parse_field(parts[0], 0, 59)?,
            hours: Self::parse_field(parts[1], 0, 23)?,
            days_of_month: Self::parse_field(parts[2], 1, 31)?,
            months: Self::parse_field(parts[3], 1, 12)?,
            days_of_week: Self::parse_field(parts[4], 0, 6)?,
        })
    }

    /// Parse a single cron field.
    fn parse_field(field: &str, min: u8, max: u8) -> Option<Vec<u8>> {
        let mut values = Vec::new();

        for part in field.split(',') {
            if part == "*" {
                // All values
                values.extend(min..=max);
            } else if let Some(step) = part.strip_prefix("*/") {
                // Step values
                let step: u8 = step.parse().ok()?;
                if step == 0 {
                    return None;
                }
                values.extend((min..=max).step_by(step as usize));
            } else if part.contains('-') {
                // Range
                let range_parts: Vec<&str> = part.split('-').collect();
                if range_parts.len() != 2 {
                    return None;
                }
                let start: u8 = range_parts[0].parse().ok()?;
                let end: u8 = range_parts[1].parse().ok()?;
                if start > end || start < min || end > max {
                    return None;
                }
                values.extend(start..=end);
            } else {
                // Single value
                let val: u8 = part.parse().ok()?;
                if val < min || val > max {
                    return None;
                }
                values.push(val);
            }
        }

        values.sort_unstable();
        values.dedup();
        Some(values)
    }

    /// Calculate the next run time.
    fn calculate_next(parsed: &ParsedCron) -> Option<u64> {
        // Simplified: return next minute that matches
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Start from next minute
        let mut timestamp = (now / 60 + 1) * 60;

        // Search for up to 1 year
        let max_iterations = 525600; // minutes in a year

        for _ in 0..max_iterations {
            let dt = timestamp_to_parts(timestamp);

            if parsed.months.contains(&dt.month)
                && parsed.days_of_month.contains(&dt.day)
                && parsed.days_of_week.contains(&dt.weekday)
                && parsed.hours.contains(&dt.hour)
                && parsed.minutes.contains(&dt.minute)
            {
                return Some(timestamp * 1000);
            }

            timestamp += 60; // Next minute
        }

        None
    }
}

/// Date/time parts.
struct DateTimeParts {
    minute: u8,
    hour: u8,
    day: u8,
    month: u8,
    weekday: u8,
}

/// Convert Unix timestamp to date parts (simplified).
fn timestamp_to_parts(timestamp: u64) -> DateTimeParts {
    // Simplified calculation - not accounting for all edge cases
    let secs = timestamp;
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    let minute = (mins % 60) as u8;
    let hour = (hours % 24) as u8;

    // Simplified day of week (Jan 1, 1970 was Thursday = 4)
    let weekday = ((days + 4) % 7) as u8;

    // Simplified month/day (very approximate)
    let year_days = days % 365;
    let month = ((year_days / 30) + 1).min(12) as u8;
    let day = ((year_days % 30) + 1).min(31) as u8;

    DateTimeParts {
        minute,
        hour,
        day,
        month,
        weekday,
    }
}

impl Schedule for CronSchedule {
    fn schedule_type(&self) -> ScheduleType {
        ScheduleType::Cron
    }

    fn next_run(&self) -> Option<u64> {
        if let Some(max) = self.max_runs {
            if self.run_count >= max {
                return None;
            }
        }
        self.next_run_at
    }

    fn advance(&mut self) {
        self.run_count += 1;
        if let Some(ref parsed) = self.parsed {
            self.next_run_at = Self::calculate_next(parsed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_parse_every_minute() {
        let schedule = CronSchedule::new("* * * * *");
        assert!(schedule.is_valid());
        assert!(schedule.next_run().is_some());
    }

    #[test]
    fn test_cron_parse_hourly() {
        let schedule = CronSchedule::hourly();
        assert!(schedule.is_valid());
        assert_eq!(schedule.expression(), "0 * * * *");
    }

    #[test]
    fn test_cron_parse_daily() {
        let schedule = CronSchedule::daily();
        assert!(schedule.is_valid());
        assert_eq!(schedule.expression(), "0 0 * * *");
    }

    #[test]
    fn test_cron_parse_daily_at() {
        let schedule = CronSchedule::daily_at(9);
        assert!(schedule.is_valid());
        assert_eq!(schedule.expression(), "0 9 * * *");
    }

    #[test]
    fn test_cron_parse_invalid() {
        let schedule = CronSchedule::new("invalid");
        assert!(!schedule.is_valid());
        assert!(schedule.next_run().is_none());
    }

    #[test]
    fn test_cron_parse_wrong_field_count() {
        let schedule = CronSchedule::new("* * *");
        assert!(!schedule.is_valid());
    }

    #[test]
    fn test_cron_with_max_runs() {
        let mut schedule = CronSchedule::every_minute().with_max_runs(2);

        schedule.advance();
        assert_eq!(schedule.run_count(), 1);
        assert!(!schedule.is_exhausted());

        schedule.advance();
        assert_eq!(schedule.run_count(), 2);
        assert!(schedule.is_exhausted());
    }

    #[test]
    fn test_cron_parse_step() {
        let schedule = CronSchedule::new("*/5 * * * *");
        assert!(schedule.is_valid());
    }

    #[test]
    fn test_cron_parse_range() {
        let schedule = CronSchedule::new("0-30 * * * *");
        assert!(schedule.is_valid());
    }

    #[test]
    fn test_cron_parse_list() {
        let schedule = CronSchedule::new("0,15,30,45 * * * *");
        assert!(schedule.is_valid());
    }

    #[test]
    fn test_parse_field_all() {
        let values = CronSchedule::parse_field("*", 0, 5).unwrap();
        assert_eq!(values, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_parse_field_step() {
        let values = CronSchedule::parse_field("*/2", 0, 6).unwrap();
        assert_eq!(values, vec![0, 2, 4, 6]);
    }

    #[test]
    fn test_parse_field_range() {
        let values = CronSchedule::parse_field("2-5", 0, 10).unwrap();
        assert_eq!(values, vec![2, 3, 4, 5]);
    }

    #[test]
    fn test_parse_field_single() {
        let values = CronSchedule::parse_field("5", 0, 10).unwrap();
        assert_eq!(values, vec![5]);
    }

    #[test]
    fn test_parse_field_invalid_range() {
        assert!(CronSchedule::parse_field("10-5", 0, 10).is_none());
    }

    #[test]
    fn test_parse_field_out_of_bounds() {
        assert!(CronSchedule::parse_field("100", 0, 59).is_none());
    }
}
