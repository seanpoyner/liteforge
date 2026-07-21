//! `forge route` - inspect and validate model-routing configs.

use crate::error::CliError;
use liteforge::model_routing::ModelRoutingConfig;
use liteforge::routing::Router;
use liteforge::{ChatCompletionRequest, Message};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(clap::Args)]
pub struct RouteArgs {
    #[command(subcommand)]
    pub command: RouteCommand,
}

#[derive(clap::Subcommand)]
pub enum RouteCommand {
    /// Show which model a prompt would route to
    ///
    /// Examples:
    ///   forge route test "what is 2+2?" --router examples/router.yaml
    ///   forge route test "prove this theorem" --router examples/router.yaml --json
    Test {
        /// The prompt to route.
        prompt: String,
        /// Path to the router YAML.
        #[arg(long, env = "FORGE_ROUTER_CONFIG")]
        router: PathBuf,
        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
    /// Parse and validate a router config, printing the resolved deployments
    Validate {
        /// Path to the router YAML.
        #[arg(long, env = "FORGE_ROUTER_CONFIG")]
        router: PathBuf,
    },
    /// List the model groups and their deployments
    List {
        /// Path to the router YAML.
        #[arg(long, env = "FORGE_ROUTER_CONFIG")]
        router: PathBuf,
    },
    /// Print where to find the MF retraining runbook (does not train)
    Train,
}

/// Load a router from YAML and attach its Layer-2 selector if configured.
pub async fn load_router(path: &Path) -> Result<Arc<Router>, CliError> {
    let yaml = std::fs::read_to_string(path).map_err(CliError::Io)?;
    let mut router = Router::from_yaml_str(&yaml)
        .map_err(|e| CliError::Config(format!("router config: {e}")))?;
    if let Some(mr) = ModelRoutingConfig::parse_optional(&yaml)
        .map_err(|e| CliError::Config(format!("model_routing config: {e}")))?
    {
        let selector = mr
            .build_selector()
            .await
            .map_err(|e| CliError::Config(format!("selector: {e}")))?;
        router = router.with_selector(Arc::from(selector));
    }
    Ok(Arc::new(router))
}

pub async fn execute(args: RouteArgs) -> Result<(), CliError> {
    match args.command {
        RouteCommand::Test {
            prompt,
            router,
            json,
        } => {
            let r = load_router(&router).await?;
            let req = ChatCompletionRequest::new("auto", vec![Message::user(prompt.clone())]);
            let decision = r
                .route_decision(&req)
                .await
                .map_err(|e| CliError::Config(format!("routing failed: {e}")))?;
            if json {
                let out = serde_json::json!({
                    "prompt": prompt,
                    "group": decision.group,
                    "model": decision.model,
                    "base_url": decision.base_url,
                    "strategy": decision.strategy,
                    "score": decision.score,
                    "fallback_chain": decision.fallback_chain,
                });
                println!("{}", serde_json::to_string_pretty(&out).unwrap());
            } else {
                println!("prompt:   {prompt}");
                println!("group:    {}", decision.group);
                println!("model:    {}", decision.model);
                println!("base_url: {}", decision.base_url);
                println!("strategy: {}", decision.strategy);
                if let Some(s) = decision.score {
                    println!("score:    {s:.3}");
                }
                println!("fallback: {}", decision.fallback_chain.join(" -> "));
            }
        }
        RouteCommand::Validate { router } => {
            let r = load_router(&router).await?;
            let mut groups = r.model_groups();
            groups.sort_unstable();
            println!("OK: router config is valid");
            println!("strategy: {}", r.strategy_name());
            println!("groups:   {}", groups.join(", "));
            let total: usize = groups.iter().map(|g| r.deployments_for(g).len()).sum();
            println!("deployments: {total}");
        }
        RouteCommand::List { router } => {
            let r = load_router(&router).await?;
            let mut groups = r.model_groups();
            groups.sort_unstable();
            for g in groups {
                println!("{g} ({} deployment(s))", r.deployments_for(g).len());
            }
        }
        RouteCommand::Train => {
            println!("MF router retraining runbook:");
            println!("  script:  scripts/retrain_mf.py  (run on a GPU host, e.g. hal-9000)");
            println!("  inputs:  RouteLLM Arena preference data + bge-m3 embeddings via LiteLLM");
            println!("  output:  mf_weights.json  (point selector.weights_path / FORGE_ROUTER_WEIGHTS at it)");
            println!("  wiki:    https://wiki.poyner.ai (Model-Routing runbook)");
        }
    }
    Ok(())
}
