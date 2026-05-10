use crate::error::CliError;
use crate::theme;
use std::path::Path;
use std::process::Command;

pub fn execute(project_dir: &Path, registry: Option<&str>) -> Result<(), CliError> {
    let config =
        super::config::AdkConfig::load(project_dir.join("adk.yaml")).map_err(CliError::Config)?;

    let image = format!("{}:latest", config.name);
    let target = if let Some(reg) = registry {
        format!("{}/{}", reg, image)
    } else {
        format!("registry.example.com/docker-local/{}", image)
    };

    println!("  {} Tagging {} as {}", theme::arrow(), image, target);
    let status = Command::new("docker")
        .args(["tag", &image, &target])
        .status()
        .map_err(CliError::Io)?;

    if !status.success() {
        return Err(CliError::Input("Docker tag failed".to_string()));
    }

    println!("  {} Pushing {}", theme::arrow(), target);
    let status = Command::new("docker")
        .args(["push", &target])
        .status()
        .map_err(CliError::Io)?;

    if status.success() {
        println!("  {} Pushed {}", theme::success("✓"), target);
        Ok(())
    } else {
        Err(CliError::Input("Docker push failed".to_string()))
    }
}
