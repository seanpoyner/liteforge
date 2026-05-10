//! Job types for scheduled execution.

use super::schedule::Schedule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Status of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is pending (waiting to run).
    #[default]
    Pending,
    /// Job is currently running.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed.
    Failed,
    /// Job was cancelled.
    Cancelled,
    /// Job is paused.
    Paused,
}

/// A scheduled job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job<S: Schedule + Clone> {
    /// Job name/ID.
    pub name: String,
    /// Job description.
    pub description: Option<String>,
    /// Current status.
    pub status: JobStatus,
    /// Schedule for the job.
    #[serde(skip)]
    pub schedule: Option<S>,
    /// Number of times the job has run.
    pub run_count: u64,
    /// Last run time (Unix timestamp ms).
    pub last_run_at: Option<u64>,
    /// Last run duration (ms).
    pub last_run_duration_ms: Option<u64>,
    /// Last error message (if failed).
    pub last_error: Option<String>,
    /// Job metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Created at timestamp.
    pub created_at: u64,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl<S: Schedule + Clone> Job<S> {
    /// Create a new job.
    pub fn new(name: impl Into<String>, schedule: S) -> Self {
        Self {
            name: name.into(),
            description: None,
            status: JobStatus::Pending,
            schedule: Some(schedule),
            run_count: 0,
            last_run_at: None,
            last_run_duration_ms: None,
            last_error: None,
            metadata: HashMap::new(),
            created_at: now_millis(),
            tags: Vec::new(),
        }
    }

    /// Set the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add metadata.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Check if job should run now.
    pub fn should_run(&self) -> bool {
        if self.status == JobStatus::Paused || self.status == JobStatus::Cancelled {
            return false;
        }
        self.schedule
            .as_ref()
            .map(|s| s.should_run())
            .unwrap_or(false)
    }

    /// Get next run time.
    pub fn next_run(&self) -> Option<u64> {
        self.schedule.as_ref().and_then(|s| s.next_run())
    }

    /// Check if job is exhausted (no more runs).
    pub fn is_exhausted(&self) -> bool {
        self.schedule
            .as_ref()
            .map(|s| s.is_exhausted())
            .unwrap_or(true)
    }

    /// Mark job as started.
    pub fn mark_started(&mut self) {
        self.status = JobStatus::Running;
    }

    /// Mark job as completed.
    pub fn mark_completed(&mut self, duration_ms: u64) {
        self.status = JobStatus::Pending; // Ready for next run
        self.run_count += 1;
        self.last_run_at = Some(now_millis());
        self.last_run_duration_ms = Some(duration_ms);
        self.last_error = None;

        if let Some(schedule) = &mut self.schedule {
            schedule.advance();
        }

        if self.is_exhausted() {
            self.status = JobStatus::Completed;
        }
    }

    /// Mark job as failed.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = JobStatus::Failed;
        self.last_run_at = Some(now_millis());
        self.last_error = Some(error.into());

        if let Some(schedule) = &mut self.schedule {
            schedule.advance();
        }
    }

    /// Pause the job.
    pub fn pause(&mut self) {
        self.status = JobStatus::Paused;
    }

    /// Resume the job.
    pub fn resume(&mut self) {
        if self.status == JobStatus::Paused {
            self.status = JobStatus::Pending;
        }
    }

    /// Cancel the job.
    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
    }

    /// Check if job has a specific tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

/// Builder for creating jobs.
pub struct JobBuilder<S: Schedule + Clone> {
    name: String,
    schedule: S,
    description: Option<String>,
    metadata: HashMap<String, String>,
    tags: Vec<String>,
}

impl<S: Schedule + Clone> JobBuilder<S> {
    /// Create a new job builder.
    pub fn new(name: impl Into<String>, schedule: S) -> Self {
        Self {
            name: name.into(),
            schedule,
            description: None,
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Set description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add metadata.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Build the job.
    pub fn build(self) -> Job<S> {
        let mut job = Job::new(self.name, self.schedule);
        job.description = self.description;
        job.metadata = self.metadata;
        job.tags = self.tags;
        job
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
    use super::super::schedule::{IntervalSchedule, OnceSchedule};
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_job_new() {
        let schedule = OnceSchedule::now();
        let job = Job::new("test_job", schedule);

        assert_eq!(job.name, "test_job");
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.run_count, 0);
    }

    #[test]
    fn test_job_with_description() {
        let schedule = OnceSchedule::now();
        let job = Job::new("test_job", schedule).description("A test job");

        assert_eq!(job.description, Some("A test job".to_string()));
    }

    #[test]
    fn test_job_should_run() {
        let schedule = OnceSchedule::now();
        let job = Job::new("test", schedule);
        assert!(job.should_run());
    }

    #[test]
    fn test_job_paused_should_not_run() {
        let schedule = OnceSchedule::now();
        let mut job = Job::new("test", schedule);
        job.pause();
        assert!(!job.should_run());
    }

    #[test]
    fn test_job_mark_completed() {
        let schedule = IntervalSchedule::new(Duration::from_secs(60));
        let mut job = Job::new("test", schedule);

        job.mark_started();
        assert_eq!(job.status, JobStatus::Running);

        job.mark_completed(100);
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.run_count, 1);
        assert_eq!(job.last_run_duration_ms, Some(100));
    }

    #[test]
    fn test_job_mark_failed() {
        let schedule = OnceSchedule::now();
        let mut job = Job::new("test", schedule);

        job.mark_failed("Something went wrong");
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.last_error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_job_exhausted() {
        let schedule = OnceSchedule::now();
        let mut job = Job::new("test", schedule);

        job.mark_completed(10);
        assert!(job.is_exhausted());
        assert_eq!(job.status, JobStatus::Completed);
    }

    #[test]
    fn test_job_pause_resume() {
        let schedule = OnceSchedule::now();
        let mut job = Job::new("test", schedule);

        job.pause();
        assert_eq!(job.status, JobStatus::Paused);

        job.resume();
        assert_eq!(job.status, JobStatus::Pending);
    }

    #[test]
    fn test_job_cancel() {
        let schedule = OnceSchedule::now();
        let mut job = Job::new("test", schedule);

        job.cancel();
        assert_eq!(job.status, JobStatus::Cancelled);
        assert!(!job.should_run());
    }

    #[test]
    fn test_job_tags() {
        let schedule = OnceSchedule::now();
        let job = Job::new("test", schedule).tag("important").tag("daily");

        assert!(job.has_tag("important"));
        assert!(job.has_tag("daily"));
        assert!(!job.has_tag("weekly"));
    }

    #[test]
    fn test_job_builder() {
        let schedule = OnceSchedule::now();
        let job = JobBuilder::new("my_job", schedule)
            .description("Test description")
            .metadata("key", "value")
            .tag("test")
            .build();

        assert_eq!(job.name, "my_job");
        assert_eq!(job.description, Some("Test description".to_string()));
        assert_eq!(job.metadata.get("key"), Some(&"value".to_string()));
        assert!(job.has_tag("test"));
    }

    #[test]
    fn test_job_builder_multiple_tags() {
        let schedule = OnceSchedule::now();
        let job = JobBuilder::new("job", schedule)
            .tags(vec!["a", "b", "c"])
            .build();

        assert_eq!(job.tags.len(), 3);
    }
}
