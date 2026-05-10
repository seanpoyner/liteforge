use crate::error::CliError;
use crate::logo;
use crate::theme;
use crate::ui;
use std::process::Command;

#[derive(clap::Args)]
pub struct InfraArgs {
    #[command(subcommand)]
    pub command: InfraCommand,
}

#[derive(clap::Subcommand)]
pub enum InfraCommand {
    /// Start infrastructure services
    Start {
        /// Run in detached mode
        #[arg(short, long)]
        detach: bool,
    },

    /// Stop infrastructure services
    Stop,

    /// Show infrastructure status
    Status,

    /// View service logs
    Logs {
        /// Service name (optional)
        service: Option<String>,

        /// Follow log output
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short, long, default_value = "100")]
        tail: String,
    },
}

const COMPOSE_CONFIG: &str = r#"
services:
  jaeger:
    image: jaegertracing/jaeger:latest
    ports:
      - "16686:16686"  # UI
      - "4317:4317"    # OTLP gRPC
      - "4318:4318"    # OTLP HTTP
    environment:
      - COLLECTOR_OTLP_ENABLED=true

  otel-collector:
    image: otel/opentelemetry-collector-contrib:latest
    command: ["--config=/etc/otel-collector-config.yaml"]
    volumes:
      - ./otel-collector-config.yaml:/etc/otel-collector-config.yaml:ro
    ports:
      - "4319:4317"    # OTLP gRPC (forwarded to 4319 to avoid conflict)
      - "4320:4318"    # OTLP HTTP
    depends_on:
      - jaeger
"#;

const OTEL_CONFIG: &str = r#"
receivers:
  otlp:
    protocols:
      grpc:
      http:

exporters:
  otlp/jaeger:
    endpoint: jaeger:4317
    tls:
      insecure: true
  debug:
    verbosity: detailed

processors:
  batch:

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [batch]
      exporters: [otlp/jaeger, debug]
    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [debug]
"#;

pub async fn execute(args: InfraArgs) -> Result<(), CliError> {
    // Check for docker compose
    let compose_cmd = find_compose_command()?;

    match args.command {
        InfraCommand::Start { detach } => start_infra(&compose_cmd, detach),
        InfraCommand::Stop => stop_infra(&compose_cmd),
        InfraCommand::Status => show_status(&compose_cmd),
        InfraCommand::Logs {
            service,
            follow,
            tail,
        } => show_logs(&compose_cmd, service, follow, &tail),
    }
}

fn find_compose_command() -> Result<Vec<String>, CliError> {
    // Try docker compose (v2)
    if Command::new("docker")
        .args(["compose", "version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Ok(vec!["docker".to_string(), "compose".to_string()]);
    }

    // Try docker-compose (v1)
    if which::which("docker-compose").is_ok() {
        return Ok(vec!["docker-compose".to_string()]);
    }

    Err(CliError::Input(
        "Docker Compose not found. Install Docker Desktop or docker-compose.".to_string(),
    ))
}

fn ensure_config_files() -> Result<std::path::PathBuf, CliError> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("forge")
        .join("infra");

    std::fs::create_dir_all(&config_dir).map_err(CliError::Io)?;

    let compose_path = config_dir.join("docker-compose.yml");
    let otel_path = config_dir.join("otel-collector-config.yaml");

    std::fs::write(&compose_path, COMPOSE_CONFIG.trim()).map_err(CliError::Io)?;
    std::fs::write(&otel_path, OTEL_CONFIG.trim()).map_err(CliError::Io)?;

    Ok(config_dir)
}

fn start_infra(compose_cmd: &[String], detach: bool) -> Result<(), CliError> {
    let config_dir = ensure_config_files()?;

    logo::print_compact();
    println!("  {}", theme::header("Starting LiteForge infrastructure..."));
    ui::kv("Config dir", &config_dir.display().to_string());
    println!();

    let mut cmd = Command::new(&compose_cmd[0]);
    if compose_cmd.len() > 1 {
        cmd.args(&compose_cmd[1..]);
    }
    cmd.current_dir(&config_dir);
    cmd.args(["up"]);

    if detach {
        cmd.arg("-d");
    }

    let status = cmd.status().map_err(CliError::Io)?;

    if detach && status.success() {
        println!();
        println!("  {}", theme::success("Infrastructure started!"));

        ui::section("Services");
        println!(
            "    {} Jaeger UI:     {}",
            theme::arrow(),
            theme::value("http://localhost:16686")
        );
        println!(
            "    {} OTLP gRPC:     {}",
            theme::arrow(),
            theme::value("localhost:4317")
        );
        println!(
            "    {} OTLP HTTP:     {}",
            theme::arrow(),
            theme::value("localhost:4318")
        );
        println!();
        println!("\n  Stop with: {}", theme::warning("forge infra stop"));
    }

    if !status.success() {
        return Err(CliError::Input(
            "Failed to start infrastructure".to_string(),
        ));
    }

    Ok(())
}

fn stop_infra(compose_cmd: &[String]) -> Result<(), CliError> {
    let config_dir = ensure_config_files()?;

    println!("  {}", theme::warning("Stopping LiteForge infrastructure..."));

    let mut cmd = Command::new(&compose_cmd[0]);
    if compose_cmd.len() > 1 {
        cmd.args(&compose_cmd[1..]);
    }
    cmd.current_dir(&config_dir);
    cmd.args(["down"]);

    let status = cmd.status().map_err(CliError::Io)?;

    if status.success() {
        println!("  {}", theme::success("Infrastructure stopped."));
    } else {
        return Err(CliError::Input("Failed to stop infrastructure".to_string()));
    }

    Ok(())
}

fn show_status(compose_cmd: &[String]) -> Result<(), CliError> {
    let config_dir = ensure_config_files()?;

    let mut cmd = Command::new(&compose_cmd[0]);
    if compose_cmd.len() > 1 {
        cmd.args(&compose_cmd[1..]);
    }
    cmd.current_dir(&config_dir);
    cmd.args(["ps"]);

    let status = cmd.status().map_err(CliError::Io)?;

    if !status.success() {
        println!("{}", theme::dimmed("No infrastructure running."));
    }

    Ok(())
}

fn show_logs(
    compose_cmd: &[String],
    service: Option<String>,
    follow: bool,
    tail: &str,
) -> Result<(), CliError> {
    let config_dir = ensure_config_files()?;

    let mut cmd = Command::new(&compose_cmd[0]);
    if compose_cmd.len() > 1 {
        cmd.args(&compose_cmd[1..]);
    }
    cmd.current_dir(&config_dir);
    cmd.args(["logs", "--tail", tail]);

    if follow {
        cmd.arg("-f");
    }

    if let Some(svc) = service {
        cmd.arg(svc);
    }

    let status = cmd.status().map_err(CliError::Io)?;

    if !status.success() {
        return Err(CliError::Input("Failed to get logs".to_string()));
    }

    Ok(())
}
