# Scheduler API

Job scheduling with interval, one-shot, and cron triggers.

## Schedule Trait

```rust
pub trait Schedule: Send + Sync {
    fn next_run(&self) -> Option<Duration>;
    fn schedule_type(&self) -> ScheduleType;
}
```

## Schedule Types

### IntervalSchedule

Repeats at a fixed interval:

```rust
use liteforge::scheduler::IntervalSchedule;
use std::time::Duration;

let schedule = IntervalSchedule::new(Duration::from_secs(60));
```

### OnceSchedule

Runs exactly once:

```rust
use liteforge::scheduler::OnceSchedule;

let schedule = OnceSchedule::new();
```

### CronSchedule

Cron expression parsing:

```rust
use liteforge::scheduler::CronSchedule;

let schedule = CronSchedule::new("0 */5 * * * *"); // Every 5 minutes
```

## Job

```rust
use liteforge::scheduler::{Job, JobBuilder};

let job = Job::builder()
    .name("sync-data")
    .schedule(IntervalSchedule::new(Duration::from_secs(300)))
    .build();
```

### JobStatus

Tracks whether a job is pending, running, completed, or failed.
