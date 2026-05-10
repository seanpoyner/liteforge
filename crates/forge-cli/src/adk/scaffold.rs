use crate::error::CliError;
use crate::theme;
use std::fs;
use std::path::Path;

pub fn execute(name: &str) -> Result<(), CliError> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        return Err(CliError::Input(format!(
            "Directory '{}' already exists",
            name
        )));
    }

    fs::create_dir_all(project_dir.join("agents")).map_err(CliError::Io)?;
    fs::create_dir_all(project_dir.join("tools")).map_err(CliError::Io)?;
    fs::create_dir_all(project_dir.join("knowledge/docs")).map_err(CliError::Io)?;
    fs::create_dir_all(project_dir.join("skills")).map_err(CliError::Io)?;
    fs::create_dir_all(project_dir.join("tests")).map_err(CliError::Io)?;

    fs::write(project_dir.join("adk.yaml"), adk_yaml(name)).map_err(CliError::Io)?;
    fs::write(project_dir.join("agents/example.yaml"), agent_yaml()).map_err(CliError::Io)?;
    fs::write(project_dir.join("tools/example_tool.py"), tool_py()).map_err(CliError::Io)?;
    fs::write(
        project_dir.join("knowledge/docs/README.md"),
        knowledge_md(name),
    )
    .map_err(CliError::Io)?;
    fs::write(project_dir.join("tests/test_example.yaml"), test_yaml()).map_err(CliError::Io)?;
    fs::write(project_dir.join(".env.example"), env_example()).map_err(CliError::Io)?;
    fs::write(project_dir.join(".dockerignore"), dockerignore()).map_err(CliError::Io)?;
    fs::write(project_dir.join(".gitignore"), gitignore()).map_err(CliError::Io)?;

    println!();
    println!(
        "  {} Created ADK project: {}",
        theme::success("✓"),
        theme::value(name)
    );
    println!();
    println!("  {}", theme::header("Project structure:"));
    println!("    {}/", name);
    println!("    ├── adk.yaml              Project manifest");
    println!("    ├── .env.example          Environment variables template");
    println!("    ├── agents/");
    println!("    │   └── example.yaml      Example agent config");
    println!("    ├── tools/");
    println!("    │   └── example_tool.py   Example Python tool");
    println!("    ├── knowledge/");
    println!("    │   └── docs/README.md    Knowledge base documents");
    println!("    ├── skills/               Custom skill definitions");
    println!("    └── tests/");
    println!("        └── test_example.yaml Eval test cases");
    println!();
    println!("  {}", theme::header("Next steps:"));
    println!("    1. cd {}", name);
    println!("    2. cp .env.example .env && edit .env");
    println!("    3. forge adk validate");
    println!("    4. forge adk dev");
    println!();

    Ok(())
}

fn adk_yaml(name: &str) -> String {
    format!(
        r#"name: {name}
version: "1.0"
description: "Agent ecosystem created with forge adk"

defaults:
  model: anthropic.claude-haiku-4-5-20251001-v1:0
  temperature: 0.7
  max_tokens: 4096

ports:
  user:
    port: 9080
    exposed: true
  mcp:
    port: 9081
    exposed: false
  tools:
    port: 9082
    exposed: false
  a2a:
    port: 9083
    exposed: true
  knowledge:
    port: 9084
    exposed: false
  skills:
    port: 9085
    exposed: false

secrets:
  provider: env
  # aws_region: us-east-1
  # aws_secret_name: my-agent/prod

knowledge:
  sources:
    - path: knowledge/docs/
      format: auto
      namespace: documentation
  # embedding_model: amazon.titan-embed-text-v2:0
  # auto_embed: true

guardrails:
  pii_detection: false
  injection_detection: false

observability:
  # otlp_endpoint: http://localhost:4317
  service_name: {name}

persistence:
  enabled: false
  path: /data

hitl:
  enabled: false
  # webhook_url: https://your-app.example.com/approvals
  # timeout_secs: 300
"#
    )
}

fn agent_yaml() -> String {
    r#"name: example
description: "An example agent with a custom Python tool"
system_prompt: |
  You are a helpful assistant. Use your tools when appropriate.
temperature: 0.7
max_tokens: 4096

tools:
  - name: example_tool
    type: python
    path: tools/example_tool.py
    description: "An example tool that echoes input"

mcp_servers: []
metadata: {}
"#
    .to_string()
}

fn tool_py() -> String {
    r#""""Example tool: echoes the input back with metadata."""


def execute(text: str) -> dict:
    """Echo the input text back with metadata.

    Args:
        text: The text to echo

    Returns:
        dict with the echoed text and metadata
    """
    return {
        "echo": text,
        "length": len(text),
        "tool": "example_tool",
    }
"#
    .to_string()
}

fn knowledge_md(name: &str) -> String {
    format!(
        r#"# {name}

This is a sample knowledge document. Place your documentation, FAQs,
and reference materials in this directory. They will be loaded into
the knowledge base when the ADK starts.

Supported formats: .md, .txt, .json
"#
    )
}

fn test_yaml() -> String {
    r#"name: example-tests
agent: example
cases:
  - name: "basic greeting"
    input: "Hello!"
    evaluators:
      - type: contains
        expected: "hello"
        case_insensitive: true

  - name: "tool usage"
    input: "Use the example_tool on the text 'test input'"
    evaluators:
      - type: contains
        expected: "echo"
"#
    .to_string()
}

fn env_example() -> String {
    r#"# LiteForge API Configuration
LITEFORGE_API_KEY=your-api-key-here
LITEFORGE_BASE_URL=https://api.example.com/v1

# Optional: Override default model
# LITEFORGE_DEFAULT_MODEL=anthropic.claude-haiku-4-5-20251001-v1:0
"#
    .to_string()
}

fn dockerignore() -> String {
    r#".git
.env
*.pyc
__pycache__
.mypy_cache
.pytest_cache
target/
node_modules/
"#
    .to_string()
}

fn gitignore() -> String {
    r#".env
*.pyc
__pycache__/
/data/
"#
    .to_string()
}
