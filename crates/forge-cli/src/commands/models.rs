use crate::error::CliError;
use crate::output::{self, OutputFormat};
use liteforge::AsyncForgeClient;

#[derive(clap::Args)]
pub struct ModelsArgs {
    #[command(subcommand)]
    pub command: ModelsCommand,

    /// Output format
    #[arg(short, long, value_enum, default_value = "pretty", global = true)]
    pub output: OutputFormat,
}

#[derive(clap::Subcommand)]
pub enum ModelsCommand {
    /// List all available models
    List,
    /// Show details for a specific model
    Info {
        /// Model ID to look up
        model_id: String,
    },
}

pub async fn execute(args: ModelsArgs) -> Result<(), CliError> {
    let client = AsyncForgeClient::new();

    match args.command {
        ModelsCommand::List => {
            let models = client.list_models().await?;
            output::print_models(&models, args.output);
        }
        ModelsCommand::Info { model_id } => {
            let models = client.list_models().await?;
            match models.find(&model_id) {
                Some(model) => {
                    let is_default = model.id == client.model();
                    output::print_model_info(model, is_default);
                }
                None => {
                    eprintln!("Model '{}' not found.\n", model_id);
                    eprintln!("Run `forge models list` to see available models.");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
