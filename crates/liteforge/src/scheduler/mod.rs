//! Scheduler and triggers for periodic agent execution.
//!
//! This module provides tools for scheduling agent execution at specific
//! times or intervals.
//!
//! # Overview
//!
//! - **Schedule**: Defines when a job should run
//! - **CronSchedule**: Cron-based scheduling
//! - **IntervalSchedule**: Fixed interval scheduling
//! - **Scheduler**: Manages and executes scheduled jobs
//!
//! # Example
//!
//! ```rust
//! use liteforge::scheduler::{IntervalSchedule, Schedule, Job};
//! use std::time::Duration;
//!
//! // Create a job that runs every 5 minutes
//! let schedule = IntervalSchedule::new(Duration::from_secs(300));
//! let job = Job::new("cleanup", schedule);
//!
//! assert_eq!(job.name, "cleanup");
//! ```

mod cron;
mod job;
mod schedule;

pub use cron::CronSchedule;
pub use job::{Job, JobBuilder, JobStatus};
pub use schedule::{IntervalSchedule, OnceSchedule, Schedule, ScheduleType};
