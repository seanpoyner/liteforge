use crate::adk;
use crate::error::CliError;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct AdkArgs {
    #[command(subcommand)]
    pub command: AdkCommand,
}

#[derive(clap::Subcommand)]
pub enum AdkCommand {
    /// Scaffold a new ADK project
    ///
    /// Creates a project directory with adk.yaml, agents, tools, knowledge,
    /// and test scaffolding.
    ///
    /// Examples:
    ///   forge adk init my-agent
    Init {
        /// Project name (creates a directory with this name)
        name: String,
    },

    /// Validate the ADK project configuration
    ///
    /// Checks adk.yaml syntax, agent files, Python tools, knowledge sources,
    /// and port conflicts.
    ///
    /// Examples:
    ///   forge adk validate
    ///   forge adk validate --dir ./my-project
    Validate {
        /// Project directory (defaults to current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },

    /// Build the ADK container image
    ///
    /// Generates a Dockerfile and builds a Docker image.
    ///
    /// Examples:
    ///   forge adk build
    ///   forge adk build --tag my-agent:v2
    Build {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        /// Image tag override
        #[arg(short, long)]
        tag: Option<String>,
    },

    /// Run the ADK container
    ///
    /// Starts the container with configured port mappings and env vars.
    ///
    /// Examples:
    ///   forge adk run
    ///   forge adk run -d
    Run {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        /// Run in detached mode
        #[arg(short = 'D', long)]
        detach: bool,
    },

    /// Start in dev mode with hot reload
    ///
    /// Runs the multi-port server locally, watching for file changes.
    ///
    /// Examples:
    ///   forge adk dev
    Dev {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },

    /// Run the eval test suite
    ///
    /// Executes test cases from tests/*.yaml against running agents.
    ///
    /// Examples:
    ///   forge adk test
    Test {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },

    /// View container logs
    ///
    /// Examples:
    ///   forge adk logs
    ///   forge adk logs -f
    Logs {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },

    /// Show container status
    ///
    /// Examples:
    ///   forge adk status
    Status {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },

    /// Stop the running container
    ///
    /// Examples:
    ///   forge adk stop
    Stop {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },

    /// Push container image to registry
    #[command(hide = true)]
    Push {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        /// Registry URL override
        #[arg(short, long)]
        registry: Option<String>,
    },
}

pub async fn execute(args: AdkArgs) -> Result<(), CliError> {
    match args.command {
        AdkCommand::Init { name } => adk::scaffold::execute(&name),
        AdkCommand::Validate { dir } => adk::validate::execute(&dir),
        AdkCommand::Build { dir, tag } => adk::build::execute(&dir, tag.as_deref()),
        AdkCommand::Run { dir, detach } => adk::run::execute(&dir, detach),
        AdkCommand::Dev { dir } => adk::dev::execute(&dir).await,
        AdkCommand::Test { dir } => adk::test::execute(&dir).await,
        AdkCommand::Logs { dir, follow } => adk::ops::logs(&dir, follow),
        AdkCommand::Status { dir } => adk::ops::status(&dir),
        AdkCommand::Stop { dir } => adk::ops::stop(&dir),
        AdkCommand::Push { dir, registry } => adk::push::execute(&dir, registry.as_deref()),
    }
}
