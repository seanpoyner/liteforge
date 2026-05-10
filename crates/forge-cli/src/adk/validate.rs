use crate::error::CliError;
use crate::theme;
use std::path::Path;

pub fn execute(project_dir: &Path) -> Result<(), CliError> {
    let adk_path = project_dir.join("adk.yaml");
    if !adk_path.exists() {
        return Err(CliError::Input(format!(
            "No adk.yaml found in {}",
            project_dir.display()
        )));
    }

    let config = super::config::AdkConfig::load(&adk_path).map_err(CliError::Config)?;

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    let agents_dir = project_dir.join("agents");
    if agents_dir.is_dir() {
        let mut found_agents = 0;
        for entry in std::fs::read_dir(&agents_dir).map_err(CliError::Io)? {
            let entry = entry.map_err(CliError::Io)?;
            let path = entry.path();
            if path
                .extension()
                .map(|e| e == "yaml" || e == "yml")
                .unwrap_or(false)
            {
                found_agents += 1;
                let content = std::fs::read_to_string(&path).map_err(CliError::Io)?;
                if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                    errors.push(format!("  {} Invalid YAML: {}", path.display(), e));
                }
            }
        }
        if found_agents == 0 {
            warnings.push("  No agent YAML files found in agents/".to_string());
        }
    } else {
        warnings.push("  agents/ directory not found".to_string());
    }

    let tools_dir = project_dir.join("tools");
    if tools_dir.is_dir() {
        for entry in std::fs::read_dir(&tools_dir).map_err(CliError::Io)? {
            let entry = entry.map_err(CliError::Io)?;
            let path = entry.path();
            if path.extension().map(|e| e == "py").unwrap_or(false) {
                let content = std::fs::read_to_string(&path).map_err(CliError::Io)?;
                if !content.contains("def execute") {
                    warnings.push(format!("  {} No execute() function found", path.display()));
                }
            }
        }
    }

    for src in &config.knowledge.sources {
        let knowledge_path = project_dir.join(&src.path);
        if !knowledge_path.exists() {
            errors.push(format!("  Knowledge source not found: {}", src.path));
        }
    }

    let ports = [
        config.ports.user.port,
        config.ports.mcp.port,
        config.ports.tools.port,
        config.ports.a2a.port,
        config.ports.knowledge.port,
        config.ports.skills.port,
    ];
    let mut seen = std::collections::HashSet::new();
    for port in &ports {
        if !seen.insert(port) {
            errors.push(format!("  Duplicate port: {}", port));
        }
    }

    if config.hitl.enabled && config.hitl.webhook_url.is_none() {
        errors.push("  HITL enabled but no webhook_url configured".to_string());
    }

    println!();
    println!(
        "  {} Validating {}",
        theme::header("ADK"),
        theme::value(&config.name)
    );
    println!();

    if !warnings.is_empty() {
        println!("  {}", theme::warning("Warnings:"));
        for w in &warnings {
            println!("    {} {}", theme::warning("!"), w);
        }
        println!();
    }

    if errors.is_empty() {
        println!("  {} Validation passed", theme::success("✓"));
        Ok(())
    } else {
        println!("  {}", theme::error_text("Errors:"));
        for e in &errors {
            println!("    {} {}", theme::error_text("✗"), e);
        }
        Err(CliError::Config("Validation failed".to_string()))
    }
}
