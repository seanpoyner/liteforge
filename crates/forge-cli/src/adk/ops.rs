use crate::error::CliError;
use crate::theme;
use std::path::Path;
use std::process::Command;

pub fn logs(project_dir: &Path, follow: bool) -> Result<(), CliError> {
    let config =
        super::config::AdkConfig::load(project_dir.join("adk.yaml")).map_err(CliError::Config)?;

    let mut args = vec!["logs".to_string(), config.name.clone()];
    if follow {
        args.push("-f".to_string());
    }

    Command::new("docker")
        .args(&args)
        .status()
        .map_err(CliError::Io)?;

    Ok(())
}

pub fn status(project_dir: &Path) -> Result<(), CliError> {
    let config =
        super::config::AdkConfig::load(project_dir.join("adk.yaml")).map_err(CliError::Config)?;

    let output = Command::new("docker")
        .args([
            "ps",
            "--filter",
            &format!("name={}", config.name),
            "--format",
            "table {{.Names}}\t{{.Status}}\t{{.Ports}}",
        ])
        .output()
        .map_err(CliError::Io)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() || stdout.lines().count() <= 1 {
        println!(
            "  {} No running container for {}",
            theme::dimmed("○"),
            config.name
        );
    } else {
        println!("{}", stdout);
    }

    Ok(())
}

pub fn stop(project_dir: &Path) -> Result<(), CliError> {
    let config =
        super::config::AdkConfig::load(project_dir.join("adk.yaml")).map_err(CliError::Config)?;

    println!("  {} Stopping {}...", theme::warning("→"), config.name);

    let status = Command::new("docker")
        .args(["stop", &config.name])
        .status()
        .map_err(CliError::Io)?;

    if status.success() {
        println!("  {} Stopped", theme::success("✓"));
    } else {
        println!(
            "  {} Container not running or already stopped",
            theme::dimmed("○")
        );
    }

    Ok(())
}
