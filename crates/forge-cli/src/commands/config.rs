use crate::error::CliError;
use crate::output;
use crate::paths;
use crate::theme;
use crate::forge_config::ForgeCliConfig;
use std::path::PathBuf;
use liteforge::ForgeConfig;

#[derive(clap::Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(clap::Subcommand)]
pub enum ConfigCommand {
    /// Display current configuration
    Show,
    /// Set a configuration value
    ///
    /// Environment values (api-key, base-url, model, timeout) are written to .env.
    /// Path values (agents-dir, skills-dir, mcp-dir, tools-dir) are written to config.toml.
    Set {
        /// Key to set
        key: String,
        /// Value to set
        value: String,
    },
    /// Get a specific configuration value
    Get {
        /// Key to get
        key: String,
    },
    /// Store a secret in the platform keyring
    ///
    /// Uses macOS Keychain, Windows Credential Manager, or Linux Secret Service.
    /// Valid keys: forge-api-key, artifactory-user, artifactory-key
    SetSecret {
        /// Secret key name
        key: String,
        /// Secret value (if omitted, prompts interactively)
        value: Option<String>,
    },
    /// Retrieve a secret from the platform keyring
    ///
    /// Outputs the secret value to stdout (for use in shell scripts).
    /// Exits with code 1 if secret is not found.
    GetSecret {
        /// Secret key name
        key: String,
    },
    /// Delete a secret from the platform keyring
    DeleteSecret {
        /// Secret key name
        key: String,
    },
    /// List all LiteForge secrets stored in the keyring
    ListSecrets,
    /// Initialize LiteForge config directory with defaults
    ///
    /// Creates the LiteForge config directory (~/.config/forge or platform equivalent)
    /// with subdirectories for agents, skills, MCP servers, and tools.
    Init,
    /// Show all paths being used
    ///
    /// Displays the effective paths for config, agents, skills, MCP, and tools.
    Paths,
}

pub async fn execute(args: ConfigArgs) -> Result<(), CliError> {
    match args.command {
        ConfigCommand::Show => execute_show(),
        ConfigCommand::Set { key, value } => execute_set(&key, &value),
        ConfigCommand::Get { key } => execute_get(&key),
        ConfigCommand::SetSecret { key, value } => execute_set_secret(&key, value),
        ConfigCommand::GetSecret { key } => execute_get_secret(&key),
        ConfigCommand::DeleteSecret { key } => execute_delete_secret(&key),
        ConfigCommand::ListSecrets => execute_list_secrets(),
        ConfigCommand::Init => execute_init(),
        ConfigCommand::Paths => execute_paths(),
    }
}

fn execute_show() -> Result<(), CliError> {
    // Show environment config
    let env_config = ForgeConfig::from_env();
    println!("{}", theme::header("Environment Settings:"));
    output::print_config(
        env_config.api_key.as_deref(),
        &env_config.base_url,
        &env_config.default_model,
        env_config.timeout.as_secs(),
    );

    // Show path config
    println!();
    println!("{}", theme::header("Path Settings:"));
    let cli_config = ForgeCliConfig::load();
    let show_path = |name: &str, path: &PathBuf, is_override: bool| {
        let suffix = if is_override {
            " (override)"
        } else {
            " (default)"
        };
        println!(
            "  {}: {}{}",
            theme::label(name),
            path.display(),
            theme::dimmed(suffix)
        );
    };

    show_path(
        "Agents",
        &cli_config.agents_dir(),
        cli_config.paths.agents_dir.is_some(),
    );
    show_path(
        "Skills",
        &cli_config.skills_dir(),
        cli_config.paths.skills_dir.is_some(),
    );
    show_path(
        "MCP",
        &cli_config.mcp_dir(),
        cli_config.paths.mcp_dir.is_some(),
    );
    show_path(
        "Tools",
        &cli_config.tools_dir(),
        cli_config.paths.tools_dir.is_some(),
    );

    Ok(())
}

fn execute_get(key: &str) -> Result<(), CliError> {
    // Check if it's a path key first
    if ForgeCliConfig::is_path_key(key) {
        let config = ForgeCliConfig::load();
        match config.get_path(key) {
            Ok(path) => println!("{}", path.display()),
            Err(e) => {
                eprintln!("{}", theme::error_text(&e.to_string()));
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // Handle environment keys
    let config = ForgeConfig::from_env();
    match key {
        "api-key" => match &config.api_key {
            Some(k) => println!("{}", k),
            None => eprintln!("{}", theme::error_text("(not set)")),
        },
        "base-url" => println!("{}", config.base_url),
        "model" => println!("{}", config.default_model),
        "timeout" => println!("{}", config.timeout.as_secs()),
        _ => {
            eprintln!("Unknown key: {}", key);
            eprintln!("Valid env keys: api-key, base-url, model, timeout");
            eprintln!("Valid path keys: agents-dir, skills-dir, mcp-dir, tools-dir");
            std::process::exit(1);
        }
    }
    Ok(())
}

fn execute_set(key: &str, value: &str) -> Result<(), CliError> {
    // Check if it's a path key first
    if ForgeCliConfig::is_path_key(key) {
        let mut config = ForgeCliConfig::load();
        let path = PathBuf::from(value);

        // Validate the path exists or can be created
        if !path.exists() {
            eprintln!(
                "{} Path does not exist: {}",
                theme::warning("Warning:"),
                path.display()
            );
            eprintln!("The path will be used when it exists.");
        }

        config.set_path(key, path).map_err(CliError::Config)?;

        config
            .save()
            .map_err(|e| CliError::Config(format!("Failed to save config: {}", e)))?;

        println!(
            "{} {} = {}",
            theme::success("Set"),
            theme::label(key),
            value
        );
        println!(
            "  {}",
            theme::dimmed(&format!("Saved to {}", paths::forge_config_path().display()))
        );
        return Ok(());
    }

    // Handle environment keys
    let env_key = match key {
        "api-key" => "LITEFORGE_API_KEY",
        "base-url" => "LITEFORGE_BASE_URL",
        "model" => "LITEFORGE_DEFAULT_MODEL",
        "timeout" => {
            if value.parse::<u64>().is_err() {
                eprintln!("Timeout must be a positive integer (seconds)");
                std::process::exit(1);
            }
            "LITEFORGE_TIMEOUT"
        }
        _ => {
            eprintln!("Unknown key: {}", key);
            eprintln!("Valid env keys: api-key, base-url, model, timeout");
            eprintln!("Valid path keys: agents-dir, skills-dir, mcp-dir, tools-dir");
            std::process::exit(1);
        }
    };

    let env_path = std::path::Path::new(".env");
    let mut contents = if env_path.exists() {
        std::fs::read_to_string(env_path)?
    } else {
        String::new()
    };

    let line = format!("{}={}", env_key, value);
    let mut found = false;
    let new_contents: Vec<String> = contents
        .lines()
        .map(|l| {
            if l.starts_with(&format!("{}=", env_key)) || l.starts_with(&format!("{}=", env_key)) {
                found = true;
                line.clone()
            } else {
                l.to_string()
            }
        })
        .collect();

    contents = new_contents.join("\n");
    if !found {
        if !contents.is_empty() && !contents.ends_with('\n') {
            contents.push('\n');
        }
        contents.push_str(&line);
        contents.push('\n');
    } else {
        if !contents.ends_with('\n') {
            contents.push('\n');
        }
    }

    std::fs::write(env_path, &contents)?;
    println!(
        "{} {} = {}",
        theme::success("Set"),
        theme::label(key),
        value
    );
    Ok(())
}

fn execute_init() -> Result<(), CliError> {
    println!("{}", theme::header("Initializing LiteForge configuration..."));

    // Create the config directory structure
    let config_dir = paths::ensure_config_dir()
        .map_err(|e| CliError::Config(format!("Failed to create config directory: {}", e)))?;

    println!("  {} {}", theme::success("Created"), config_dir.display());

    // Create default config.toml if it doesn't exist
    let config_path = paths::forge_config_path();
    if !config_path.exists() {
        let default_content = crate::forge_config::default_config_content();
        std::fs::write(&config_path, default_content)
            .map_err(|e| CliError::Config(format!("Failed to write config file: {}", e)))?;
        println!("  {} {}", theme::success("Created"), config_path.display());
    } else {
        println!(
            "  {} {} (already exists)",
            theme::warning("Skipped"),
            config_path.display()
        );
    }

    // Show subdirectories
    println!();
    println!("{}", theme::header("Subdirectories:"));
    for subdir in ["agents", "skills", "mcp", "tools"] {
        let path = config_dir.join(subdir);
        println!("  {} {}/", theme::success("✓"), path.display());
    }

    // Show data directory
    println!();
    println!("{}", theme::header("Data directory:"));
    let data_dir = paths::ensure_data_dir()
        .map_err(|e| CliError::Config(format!("Failed to create data directory: {}", e)))?;
    println!("  {} {}/", theme::success("✓"), data_dir.display());

    println!();
    println!(
        "{}",
        theme::success("LiteForge configuration initialized successfully!")
    );
    println!();
    println!("Next steps:");
    println!(
        "  • Add agents to: {}/",
        paths::default_agents_dir().display()
    );
    println!(
        "  • Add skills to: {}/",
        paths::default_skills_dir().display()
    );
    println!(
        "  • Configure MCP servers in: {}/",
        paths::default_mcp_dir().display()
    );

    Ok(())
}

fn execute_paths() -> Result<(), CliError> {
    let cli_config = ForgeCliConfig::load();

    println!("{}", theme::header("LiteForge Paths:"));
    println!();

    // Config directory
    println!(
        "  {}: {}",
        theme::label("Config dir"),
        paths::forge_config_dir().display()
    );
    println!(
        "  {}: {}",
        theme::label("Config file"),
        paths::forge_config_path().display()
    );

    println!();
    println!("{}", theme::header("Resource directories:"));

    let show_path = |name: &str, path: &PathBuf, override_path: &Option<PathBuf>| {
        let exists = path.exists();
        let exists_marker = if exists {
            theme::success("✓")
        } else {
            theme::error_text("✗")
        };
        let override_note = if override_path.is_some() {
            theme::warning(" (override)")
        } else {
            theme::dimmed(" (default)")
        };
        println!(
            "  {} {}: {}{}",
            exists_marker,
            theme::label(name),
            path.display(),
            override_note
        );
    };

    show_path(
        "Agents",
        &cli_config.agents_dir(),
        &cli_config.paths.agents_dir,
    );
    show_path(
        "Skills",
        &cli_config.skills_dir(),
        &cli_config.paths.skills_dir,
    );
    show_path("MCP", &cli_config.mcp_dir(), &cli_config.paths.mcp_dir);
    show_path(
        "Tools",
        &cli_config.tools_dir(),
        &cli_config.paths.tools_dir,
    );

    println!();
    println!("{}", theme::header("Data directory:"));
    let data_dir = paths::forge_data_dir();
    let exists_marker = if data_dir.exists() {
        theme::success("✓")
    } else {
        theme::error_text("✗")
    };
    println!(
        "  {} {}: {}",
        exists_marker,
        theme::label("Data"),
        data_dir.display()
    );
    println!(
        "  {} {}: {}",
        exists_marker,
        theme::label("Usage DB"),
        paths::usage_db_path().display()
    );

    Ok(())
}

// Keyring service name for LiteForge secrets
const KEYRING_SERVICE: &str = "forge";

// Valid secret keys
const VALID_SECRET_KEYS: &[&str] = &["forge-api-key", "artifactory-user", "artifactory-key"];

fn validate_secret_key(key: &str) -> Result<(), CliError> {
    if !VALID_SECRET_KEYS.contains(&key) {
        return Err(CliError::Config(format!(
            "Invalid secret key: {}. Valid keys: {}",
            key,
            VALID_SECRET_KEYS.join(", ")
        )));
    }
    Ok(())
}

fn execute_set_secret(key: &str, value: Option<String>) -> Result<(), CliError> {
    validate_secret_key(key)?;

    let secret_value = match value {
        Some(v) => v,
        None => {
            // Prompt for secret interactively
            eprint!("Enter value for {}: ", theme::label(key));
            let mut input = String::new();
            // Read from stdin without echoing (basic approach)
            std::io::stdin()
                .read_line(&mut input)
                .map_err(|e| CliError::Config(format!("Failed to read input: {}", e)))?;
            input.trim().to_string()
        }
    };

    if secret_value.is_empty() {
        return Err(CliError::Config("Secret value cannot be empty".to_string()));
    }

    let entry = keyring::Entry::new(KEYRING_SERVICE, key)
        .map_err(|e| CliError::Config(format!("Failed to create keyring entry: {}", e)))?;

    entry
        .set_password(&secret_value)
        .map_err(|e| CliError::Config(format!("Failed to store secret: {}", e)))?;

    println!(
        "{} Secret '{}' stored in keyring",
        theme::success("✓"),
        theme::label(key)
    );
    Ok(())
}

fn execute_get_secret(key: &str) -> Result<(), CliError> {
    validate_secret_key(key)?;

    let entry = keyring::Entry::new(KEYRING_SERVICE, key)
        .map_err(|e| CliError::Config(format!("Failed to access keyring: {}", e)))?;

    match entry.get_password() {
        Ok(password) => {
            // Output only the secret (for use in shell scripts)
            print!("{}", password);
            Ok(())
        }
        Err(keyring::Error::NoEntry) => {
            // Exit with code 1 if not found (silent for scripting)
            std::process::exit(1);
        }
        Err(e) => Err(CliError::Config(format!(
            "Failed to retrieve secret: {}",
            e
        ))),
    }
}

fn execute_delete_secret(key: &str) -> Result<(), CliError> {
    validate_secret_key(key)?;

    let entry = keyring::Entry::new(KEYRING_SERVICE, key)
        .map_err(|e| CliError::Config(format!("Failed to access keyring: {}", e)))?;

    match entry.delete_credential() {
        Ok(()) => {
            println!(
                "{} Secret '{}' deleted from keyring",
                theme::success("✓"),
                theme::label(key)
            );
            Ok(())
        }
        Err(keyring::Error::NoEntry) => {
            println!(
                "{} Secret '{}' not found in keyring",
                theme::warning("!"),
                theme::label(key)
            );
            Ok(())
        }
        Err(e) => Err(CliError::Config(format!("Failed to delete secret: {}", e))),
    }
}

fn execute_list_secrets() -> Result<(), CliError> {
    println!("{}", theme::header("LiteForge Secrets:"));
    println!();

    for key in VALID_SECRET_KEYS {
        let entry = keyring::Entry::new(KEYRING_SERVICE, key)
            .map_err(|e| CliError::Config(format!("Failed to access keyring: {}", e)))?;

        let status = match entry.get_password() {
            Ok(_) => format!("{} (set)", theme::success("✓")),
            Err(keyring::Error::NoEntry) => format!("{} (not set)", theme::error_text("✗")),
            Err(_) => format!("{} (error)", theme::warning("?")),
        };

        println!("  {}: {}", theme::label(key), status);
    }

    println!();
    println!(
        "{}",
        theme::dimmed("Use 'forge config set-secret <key>' to store a secret.")
    );

    Ok(())
}
