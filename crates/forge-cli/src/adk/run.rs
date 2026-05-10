use crate::error::CliError;
use crate::theme;
use std::path::Path;
use std::process::Command;

pub fn execute(project_dir: &Path, detach: bool) -> Result<(), CliError> {
    let adk_path = project_dir.join("adk.yaml");
    let config = super::config::AdkConfig::load(&adk_path).map_err(CliError::Config)?;

    let image_tag = format!("{}:latest", config.name);
    let container_name = config.name.clone();

    println!();
    println!(
        "  {} Running ADK container: {}",
        theme::header("ADK"),
        theme::value(&config.name)
    );
    println!();

    let mut args = vec![
        "run".to_string(),
        "--name".to_string(),
        container_name.clone(),
        "--rm".to_string(),
    ];

    for port in config.ports.exposed_ports() {
        args.push("-p".to_string());
        args.push(format!("{}:{}", port, port));
    }

    let env_path = project_dir.join(".env");
    if env_path.exists() {
        args.push("--env-file".to_string());
        args.push(env_path.display().to_string());
    }

    if config.persistence.enabled {
        args.push("-v".to_string());
        args.push(format!("{}-data:{}", config.name, config.persistence.path));
    }

    if detach {
        args.push("-d".to_string());
    }

    args.push(image_tag.clone());

    println!("  {} docker {}", theme::arrow(), args.join(" "));
    println!();

    let status = Command::new("docker")
        .args(&args)
        .status()
        .map_err(CliError::Io)?;

    if detach && status.success() {
        println!(
            "  {} Container started: {}",
            theme::success("✓"),
            theme::value(&container_name)
        );
        for port in config.ports.exposed_ports() {
            println!("    {} http://localhost:{}", theme::arrow(), port);
        }
        println!();
        println!("  Stop with: {}", theme::value("forge adk stop"));
    }

    if !status.success() {
        return Err(CliError::Input("Docker run failed".to_string()));
    }

    Ok(())
}
