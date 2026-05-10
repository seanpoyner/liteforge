use crate::error::CliError;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum PythonToolKind {
    Function,
    McpStdio,
    Script,
}

#[derive(Debug, Clone)]
pub struct DiscoveredTool {
    pub name: String,
    pub description: String,
    pub kind: PythonToolKind,
    pub path: PathBuf,
    pub parameters: Value,
}

pub fn discover_tools(tools_dir: &Path) -> Result<Vec<DiscoveredTool>, CliError> {
    if !tools_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut tools = Vec::new();

    for entry in std::fs::read_dir(tools_dir).map_err(CliError::Io)? {
        let entry = entry.map_err(CliError::Io)?;
        let path = entry.path();

        if path.extension().map(|e| e == "py").unwrap_or(false) {
            if let Some(tool) = parse_python_tool(&path)? {
                tools.push(tool);
            }
        }
    }

    Ok(tools)
}

fn parse_python_tool(path: &Path) -> Result<Option<DiscoveredTool>, CliError> {
    let content = std::fs::read_to_string(path).map_err(CliError::Io)?;
    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let kind = if file_stem.starts_with("mcp_") {
        PythonToolKind::McpStdio
    } else if file_stem.starts_with("script_") {
        PythonToolKind::Script
    } else if content.contains("def execute") {
        PythonToolKind::Function
    } else {
        return Ok(None);
    };

    let description =
        extract_module_docstring(&content).unwrap_or_else(|| format!("Python tool: {}", file_stem));

    let parameters = extract_parameters(&content);

    Ok(Some(DiscoveredTool {
        name: file_stem.to_string(),
        description,
        kind,
        path: path.to_path_buf(),
        parameters,
    }))
}

fn extract_module_docstring(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix("\"\"\"") {
        if let Some(end) = rest.find("\"\"\"") {
            return Some(rest[..end].trim().to_string());
        }
    }
    None
}

fn extract_parameters(content: &str) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("def execute(") {
            let params_str = trimmed
                .trim_start_matches("def execute(")
                .trim_end_matches("):")
                .trim_end_matches(") -> dict:")
                .trim_end_matches(") -> str:");

            for param in params_str.split(',') {
                let param = param.trim();
                if param.is_empty() || param == "self" {
                    continue;
                }

                let (name, type_hint) = if param.contains(':') {
                    let parts: Vec<&str> = param.splitn(2, ':').collect();
                    (parts[0].trim(), Some(parts[1].trim()))
                } else if param.contains('=') {
                    let parts: Vec<&str> = param.splitn(2, '=').collect();
                    (parts[0].trim(), None)
                } else {
                    (param, None)
                };

                let name = name.trim_end_matches(':');

                if name.contains('=') {
                    let parts: Vec<&str> = name.splitn(2, '=').collect();
                    let prop_name = parts[0].trim();
                    properties.insert(prop_name.to_string(), serde_json::json!({"type": "string"}));
                } else {
                    let json_type = match type_hint {
                        Some(t) if t.starts_with("int") => "integer",
                        Some(t) if t.starts_with("float") => "number",
                        Some(t) if t.starts_with("bool") => "boolean",
                        Some(t) if t.starts_with("list") || t.starts_with("List") => "array",
                        Some(t) if t.starts_with("dict") || t.starts_with("Dict") => "object",
                        _ => "string",
                    };

                    properties.insert(name.to_string(), serde_json::json!({"type": json_type}));

                    if !param.contains('=') {
                        required.push(serde_json::Value::String(name.to_string()));
                    }
                }
            }
            break;
        }
    }

    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

pub fn execute_python_tool(tool: &DiscoveredTool, arguments: &Value) -> Result<Value, String> {
    match tool.kind {
        PythonToolKind::Function => execute_function_tool(tool, arguments),
        PythonToolKind::Script => execute_script_tool(tool, arguments),
        PythonToolKind::McpStdio => {
            Err("MCP stdio tools must be launched via MCP manager".to_string())
        }
    }
}

fn execute_function_tool(tool: &DiscoveredTool, arguments: &Value) -> Result<Value, String> {
    let args_json = serde_json::to_string(arguments).unwrap_or_default();

    let wrapper = format!(
        r#"
import json, sys
sys.path.insert(0, '.')
args = json.loads('{args_json}')
from {module} import execute
result = execute(**args)
print(json.dumps(result))
"#,
        args_json = args_json.replace('\'', "\\'"),
        module = tool.name,
    );

    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(&wrapper)
        .current_dir(
            tool.path
                .parent()
                .and_then(|p| p.parent())
                .unwrap_or(Path::new(".")),
        )
        .output()
        .map_err(|e| format!("Failed to execute Python tool: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Python tool error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).map_err(|e| format!("Failed to parse tool output: {}", e))
}

fn execute_script_tool(tool: &DiscoveredTool, arguments: &Value) -> Result<Value, String> {
    let input = serde_json::to_string(arguments).unwrap_or_default();

    let output = std::process::Command::new("python3")
        .arg(&tool.path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                stdin.write_all(input.as_bytes()).ok();
            }
            child.wait_with_output()
        })
        .map_err(|e| format!("Failed to execute script tool: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Script tool error: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).map_err(|e| format!("Failed to parse script output: {}", e))
}
