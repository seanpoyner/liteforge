use crate::error::CliError;
use crate::theme;
use std::path::Path;
use std::process::Command;

pub fn execute(project_dir: &Path, tag: Option<&str>) -> Result<(), CliError> {
    let adk_path = project_dir.join("adk.yaml");
    let config = super::config::AdkConfig::load(&adk_path).map_err(CliError::Config)?;

    println!();
    println!(
        "  {} Building ADK container: {}",
        theme::header("ADK"),
        theme::value(&config.name)
    );
    println!();

    super::dockerfile::write_dockerfile(&config, project_dir)?;

    let image_tag = tag
        .map(|t| t.to_string())
        .unwrap_or_else(|| format!("{}:latest", config.name));

    println!(
        "  {} Building image: {}",
        theme::arrow(),
        theme::value(&image_tag)
    );

    let status = Command::new("docker")
        .args(["build", "-t", &image_tag, "."])
        .current_dir(project_dir)
        .status()
        .map_err(CliError::Io)?;

    if status.success() {
        println!();
        println!(
            "  {} Image built: {}",
            theme::success("✓"),
            theme::value(&image_tag)
        );
        println!();
        println!("  Run with: {}", theme::value("forge adk run"));
    } else {
        return Err(CliError::Input("Docker build failed".to_string()));
    }

    Ok(())
}
