use crate::error::CliError;
use crate::output::OutputFormat;
use crate::theme;
use crate::usage::{ensure_db_dir, UsageDb};
use chrono::{Datelike, Duration, TimeZone, Utc};
use comfy_table::{presets, Table};

#[derive(clap::Args)]
pub struct UsageArgs {
    /// Time period
    #[arg(short, long, value_enum, default_value = "monthly")]
    pub period: Period,

    /// Number of periods to show
    #[arg(short = 'n', long, default_value = "1")]
    pub count: u32,

    /// Show breakdown by model
    #[arg(long)]
    pub by_model: bool,

    /// Show session list
    #[arg(long)]
    pub sessions: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "pretty")]
    pub output: OutputFormat,

    /// Export as CSV
    #[arg(long)]
    pub csv: bool,
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Period {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

pub async fn execute(args: UsageArgs) -> Result<(), CliError> {
    let db_path = ensure_db_dir().map_err(CliError::Io)?;

    if !db_path.exists() {
        println!("No usage data found.");
        println!("Usage tracking will start when you use `forge claude` to launch Claude Code.");
        return Ok(());
    }

    let db = UsageDb::open(&db_path).map_err(|e| CliError::Input(e.to_string()))?;

    let now = Utc::now();
    let (start, end) = get_period_range(now, args.period, args.count);

    if args.sessions {
        show_sessions(&db, start, end, &args)?;
    } else if args.by_model {
        show_by_model(&db, start, end, &args)?;
    } else {
        show_summary(&db, start, end, &args)?;
    }

    Ok(())
}

fn get_period_range(
    now: chrono::DateTime<Utc>,
    period: Period,
    count: u32,
) -> (chrono::DateTime<Utc>, chrono::DateTime<Utc>) {
    let end = now;
    let start = match period {
        Period::Daily => end - Duration::days(count as i64),
        Period::Weekly => end - Duration::weeks(count as i64),
        Period::Monthly => {
            let months = count as i32;
            let year = end.year();
            let month = end.month() as i32;
            let new_month = month - months;
            let (year, month) = if new_month <= 0 {
                (
                    year - 1 + (new_month - 1) / 12,
                    ((new_month - 1) % 12 + 12) as u32 + 1,
                )
            } else {
                (year, new_month as u32)
            };
            Utc.with_ymd_and_hms(year, month, end.day().min(28), 0, 0, 0)
                .single()
                .unwrap_or(end)
        }
        Period::Quarterly => {
            let months = (count * 3) as i32;
            let year = end.year();
            let month = end.month() as i32;
            let new_month = month - months;
            let (year, month) = if new_month <= 0 {
                (
                    year - 1 + (new_month - 1) / 12,
                    ((new_month - 1) % 12 + 12) as u32 + 1,
                )
            } else {
                (year, new_month as u32)
            };
            Utc.with_ymd_and_hms(year, month, end.day().min(28), 0, 0, 0)
                .single()
                .unwrap_or(end)
        }
        Period::Yearly => end - Duration::days(365 * count as i64),
    };
    (start, end)
}

fn show_summary(
    db: &UsageDb,
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
    args: &UsageArgs,
) -> Result<(), CliError> {
    let summary = db
        .get_usage_summary(start, end)
        .map_err(|e| CliError::Input(e.to_string()))?;

    match args.output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
        }
        OutputFormat::Raw => {
            println!("requests={}", summary.request_count);
            println!("sessions={}", summary.session_count);
            println!("total_tokens={}", summary.total_tokens);
            println!("prompt_tokens={}", summary.total_prompt_tokens);
            println!("completion_tokens={}", summary.total_completion_tokens);
            if let Some(latency) = summary.avg_latency_ms {
                println!("avg_latency_ms={:.0}", latency);
            }
        }
        OutputFormat::Pretty => {
            println!("{}", theme::header("Usage Summary"));
            println!(
                "Period: {} to {}",
                start.format("%Y-%m-%d"),
                end.format("%Y-%m-%d")
            );
            println!();

            let mut table = Table::new();
            table.load_preset(presets::UTF8_FULL_CONDENSED);
            table.add_row(vec!["Requests", &summary.request_count.to_string()]);
            table.add_row(vec!["Sessions", &summary.session_count.to_string()]);
            table.add_row(vec!["Total Tokens", &format_number(summary.total_tokens)]);
            table.add_row(vec![
                "Prompt Tokens",
                &format_number(summary.total_prompt_tokens),
            ]);
            table.add_row(vec![
                "Completion Tokens",
                &format_number(summary.total_completion_tokens),
            ]);
            if let Some(latency) = summary.avg_latency_ms {
                table.add_row(vec!["Avg Latency", &format!("{:.0}ms", latency)]);
            }

            println!("{table}");
        }
    }

    Ok(())
}

fn show_by_model(
    db: &UsageDb,
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
    args: &UsageArgs,
) -> Result<(), CliError> {
    let models = db
        .get_usage_by_model(start, end)
        .map_err(|e| CliError::Input(e.to_string()))?;

    if models.is_empty() {
        println!("No usage data for this period.");
        return Ok(());
    }

    match args.output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&models).unwrap());
        }
        OutputFormat::Raw => {
            for m in &models {
                println!("{}\t{}\t{}", m.model, m.request_count, m.total_tokens);
            }
        }
        OutputFormat::Pretty => {
            println!("{}", theme::header("Usage by Model"));
            println!(
                "Period: {} to {}",
                start.format("%Y-%m-%d"),
                end.format("%Y-%m-%d")
            );
            println!();

            let mut table = Table::new();
            table.load_preset(presets::UTF8_FULL_CONDENSED);
            table.set_header(vec!["Model", "Requests", "Prompt", "Completion", "Total"]);

            for m in &models {
                table.add_row(vec![
                    m.model.clone(),
                    m.request_count.to_string(),
                    format_number(m.prompt_tokens),
                    format_number(m.completion_tokens),
                    format_number(m.total_tokens),
                ]);
            }

            println!("{table}");
        }
    }

    if args.csv {
        println!("\n{}", theme::label("CSV Export:"));
        println!("model,requests,prompt_tokens,completion_tokens,total_tokens");
        for m in &models {
            println!(
                "{},{},{},{},{}",
                m.model, m.request_count, m.prompt_tokens, m.completion_tokens, m.total_tokens
            );
        }
    }

    Ok(())
}

fn show_sessions(
    db: &UsageDb,
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
    args: &UsageArgs,
) -> Result<(), CliError> {
    let sessions = db
        .get_sessions(start, end)
        .map_err(|e| CliError::Input(e.to_string()))?;

    if sessions.is_empty() {
        println!("No sessions for this period.");
        return Ok(());
    }

    match args.output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&sessions).unwrap());
        }
        OutputFormat::Raw => {
            for s in &sessions {
                println!(
                    "{}\t{}\t{}\t{}",
                    s.id, s.tool, s.request_count, s.total_tokens
                );
            }
        }
        OutputFormat::Pretty => {
            println!("{}", theme::header("Sessions"));
            println!(
                "Period: {} to {}",
                start.format("%Y-%m-%d"),
                end.format("%Y-%m-%d")
            );
            println!();

            let mut table = Table::new();
            table.load_preset(presets::UTF8_FULL_CONDENSED);
            table.set_header(vec!["ID", "Tool", "Started", "Requests", "Tokens"]);

            for s in &sessions {
                table.add_row(vec![
                    truncate(&s.id, 12),
                    s.tool.clone(),
                    s.started_at.format("%Y-%m-%d %H:%M").to_string(),
                    s.request_count.to_string(),
                    format_number(s.total_tokens),
                ]);
            }

            println!("{table}");
            println!(
                "\n{}",
                theme::dimmed(&format!("{} sessions", sessions.len()))
            );
        }
    }

    Ok(())
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
