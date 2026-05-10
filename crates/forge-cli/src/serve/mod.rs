pub mod a2a;
pub mod config;
pub mod knowledge_serve;
pub mod mcp_serve;
pub mod skills_serve;
pub mod state;
pub mod tools_serve;
pub mod user;

use crate::error::CliError;
use crate::logo;
use crate::theme;
use crate::ui;
use config::{RoleConfig, ServeConfig};
use state::AppState;
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::sync::Arc;

fn find_open_port(cfg: &RoleConfig) -> Result<(SocketAddr, StdTcpListener), CliError> {
    let addr: SocketAddr = cfg
        .addr()
        .parse()
        .map_err(|e| CliError::Input(format!("Invalid address: {}", e)))?;

    match StdTcpListener::bind(addr) {
        Ok(listener) => Ok((addr, listener)),
        Err(_) => {
            let fallback: SocketAddr = format!("{}:0", cfg.host)
                .parse()
                .map_err(|e| CliError::Input(format!("Invalid host: {}", e)))?;
            let listener = StdTcpListener::bind(fallback).map_err(CliError::Io)?;
            let actual = listener.local_addr().map_err(CliError::Io)?;
            Ok((actual, listener))
        }
    }
}

async fn serve_from_listener(
    std_listener: StdTcpListener,
    router: axum::Router,
) -> Result<(), CliError> {
    std_listener.set_nonblocking(true).map_err(CliError::Io)?;
    let listener = tokio::net::TcpListener::from_std(std_listener).map_err(CliError::Io)?;
    axum::serve(listener, router)
        .await
        .map_err(|e| CliError::Input(format!("Server error: {}", e)))?;
    Ok(())
}

struct BoundRole {
    name: &'static str,
    requested: SocketAddr,
    actual: SocketAddr,
    listener: StdTcpListener,
    enabled: bool,
}

impl BoundRole {
    fn was_reassigned(&self) -> bool {
        self.requested.port() != self.actual.port()
    }
}

fn print_bound_role(role: &BoundRole) {
    if !role.enabled {
        println!(
            "    {} {:<12} {}",
            theme::dimmed("○"),
            theme::dimmed(role.name),
            theme::dimmed("disabled")
        );
        return;
    }

    let url = format!("http://{}", role.actual);
    if role.was_reassigned() {
        println!(
            "    {} {:<12} {} {}",
            theme::arrow(),
            role.name,
            theme::value(&url),
            theme::warning(&format!("(wanted :{}, in use)", role.requested.port())),
        );
    } else {
        println!(
            "    {} {:<12} {}",
            theme::arrow(),
            role.name,
            theme::value(&url),
        );
    }
}

fn bind_role(name: &'static str, cfg: &RoleConfig) -> Result<BoundRole, CliError> {
    if !cfg.enabled {
        let addr: SocketAddr = cfg
            .addr()
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], cfg.port)));
        return Ok(BoundRole {
            name,
            requested: addr,
            actual: addr,
            listener: StdTcpListener::bind("127.0.0.1:0").map_err(CliError::Io)?,
            enabled: false,
        });
    }

    let (actual, listener) = find_open_port(cfg)?;
    let requested: SocketAddr = cfg.addr().parse().unwrap_or(actual);
    Ok(BoundRole {
        name,
        requested,
        actual,
        listener,
        enabled: true,
    })
}

pub async fn start_all(state: Arc<AppState>, config: &ServeConfig) -> Result<(), CliError> {
    let roles = [
        ("user", &config.user),
        ("mcp", &config.mcp),
        ("tools", &config.tools),
        ("a2a", &config.a2a),
        ("knowledge", &config.knowledge),
        ("skills", &config.skills),
    ];

    let mut bound: Vec<BoundRole> = Vec::new();
    for &(name, cfg) in &roles {
        let static_name: &'static str = match name {
            "user" => "user",
            "mcp" => "mcp",
            "tools" => "tools",
            "a2a" => "a2a",
            "knowledge" => "knowledge",
            "skills" => "skills",
            _ => unreachable!(),
        };
        bound.push(bind_role(static_name, cfg)?);
    }

    logo::print_compact();
    println!("  {}", theme::header("LiteForge Multi-Port Server"));
    println!();
    ui::section("Servers");
    for role in &bound {
        print_bound_role(role);
    }
    println!();
    println!("  Press {} to stop all servers.", theme::warning("Ctrl+C"));
    println!();

    let mut handles = Vec::new();

    for role in bound {
        if !role.enabled {
            continue;
        }
        let s = state.clone();
        let router = match role.name {
            "user" => user::router(s),
            "mcp" => mcp_serve::router(s),
            "tools" => tools_serve::router(s),
            "a2a" => a2a::router(s),
            "knowledge" => knowledge_serve::router(s),
            "skills" => skills_serve::router(s),
            _ => continue,
        };
        handles.push(tokio::spawn(async move {
            serve_from_listener(role.listener, router).await
        }));
    }

    if handles.is_empty() {
        return Err(CliError::Input("No servers enabled".to_string()));
    }

    let (result, _, _) = futures::future::select_all(handles).await;
    result.map_err(|e| CliError::Input(format!("Server task panicked: {}", e)))?
}

pub async fn start_single(
    state: Arc<AppState>,
    role_name: &str,
    config: &ServeConfig,
) -> Result<(), CliError> {
    let cfg = match role_name {
        "user" => &config.user,
        "mcp" => &config.mcp,
        "tools" => &config.tools,
        "a2a" => &config.a2a,
        "knowledge" => &config.knowledge,
        "skills" => &config.skills,
        _ => return Err(CliError::Input(format!("Unknown role: {}", role_name))),
    };

    let static_name: &'static str = match role_name {
        "user" => "user",
        "mcp" => "mcp",
        "tools" => "tools",
        "a2a" => "a2a",
        "knowledge" => "knowledge",
        "skills" => "skills",
        _ => unreachable!(),
    };

    let role = bind_role(static_name, cfg)?;

    logo::print_compact();
    println!("  {}", theme::header(&format!("LiteForge {} Server", role_name)));
    print_bound_role(&role);
    println!();

    let router = match role_name {
        "user" => user::router(state),
        "mcp" => mcp_serve::router(state),
        "tools" => tools_serve::router(state),
        "a2a" => a2a::router(state),
        "knowledge" => knowledge_serve::router(state),
        "skills" => skills_serve::router(state),
        _ => unreachable!(),
    };

    serve_from_listener(role.listener, router).await
}
