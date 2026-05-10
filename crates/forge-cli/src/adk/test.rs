use crate::error::CliError;
use crate::theme;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct TestSuite {
    name: String,
    agent: String,
    cases: Vec<TestCase>,
}

#[derive(Deserialize)]
struct TestCase {
    name: String,
    input: String,
    evaluators: Vec<Evaluator>,
}

#[derive(Deserialize)]
struct Evaluator {
    #[serde(rename = "type")]
    eval_type: String,
    #[serde(default)]
    expected: Option<String>,
    #[serde(default)]
    pattern: Option<String>,
    #[serde(default)]
    case_insensitive: Option<bool>,
}

pub async fn execute(project_dir: &Path) -> Result<(), CliError> {
    let tests_dir = project_dir.join("tests");
    if !tests_dir.is_dir() {
        return Err(CliError::Input("No tests/ directory found".to_string()));
    }

    let adk_path = project_dir.join("adk.yaml");
    let config = super::config::AdkConfig::load(&adk_path).map_err(CliError::Config)?;

    let mut suites = Vec::new();
    for entry in std::fs::read_dir(&tests_dir).map_err(CliError::Io)? {
        let entry = entry.map_err(CliError::Io)?;
        let path = entry.path();
        if path
            .extension()
            .map(|e| e == "yaml" || e == "yml")
            .unwrap_or(false)
        {
            let content = std::fs::read_to_string(&path).map_err(CliError::Io)?;
            match serde_yaml::from_str::<TestSuite>(&content) {
                Ok(suite) => suites.push(suite),
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                }
            }
        }
    }

    if suites.is_empty() {
        return Err(CliError::Input(
            "No test suites found in tests/".to_string(),
        ));
    }

    println!();
    println!(
        "  {} Running eval suite for {}",
        theme::header("ADK Test"),
        theme::value(&config.name)
    );
    println!();

    let base_url = format!("http://127.0.0.1:{}", config.ports.user.port);
    let client = reqwest::Client::new();

    let mut total_pass = 0;
    let mut total_fail = 0;

    for suite in &suites {
        println!("  {} {}", theme::subheader("Suite:"), suite.name);

        for case in &suite.cases {
            let url = format!("{}/v1/agents/{}/chat", base_url, suite.agent);
            let body = serde_json::json!({
                "messages": [{"role": "user", "content": case.input}]
            });

            let result = client.post(&url).json(&body).send().await;

            match result {
                Ok(resp) if resp.status().is_success() => {
                    let json: serde_json::Value = resp.json().await.unwrap_or_default();
                    let content = json
                        .pointer("/choices/0/message/content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let passed = evaluate_all(&case.evaluators, content);
                    if passed {
                        total_pass += 1;
                        println!("    {} {}", theme::success("PASS"), case.name);
                    } else {
                        total_fail += 1;
                        println!("    {} {}", theme::error_text("FAIL"), case.name);
                    }
                }
                Ok(resp) => {
                    total_fail += 1;
                    println!(
                        "    {} {} (HTTP {})",
                        theme::error_text("FAIL"),
                        case.name,
                        resp.status()
                    );
                }
                Err(e) => {
                    total_fail += 1;
                    println!("    {} {} ({})", theme::error_text("FAIL"), case.name, e);
                }
            }
        }
        println!();
    }

    println!(
        "  Results: {} passed, {} failed",
        theme::success(&total_pass.to_string()),
        if total_fail > 0 {
            theme::error_text(&total_fail.to_string())
        } else {
            theme::dimmed("0")
        }
    );
    println!();

    if total_fail > 0 {
        Err(CliError::Input(format!("{} test(s) failed", total_fail)))
    } else {
        Ok(())
    }
}

fn evaluate_all(evaluators: &[Evaluator], content: &str) -> bool {
    evaluators.iter().all(|e| evaluate_one(e, content))
}

fn evaluate_one(eval: &Evaluator, content: &str) -> bool {
    match eval.eval_type.as_str() {
        "contains" => {
            if let Some(expected) = &eval.expected {
                if eval.case_insensitive.unwrap_or(false) {
                    content.to_lowercase().contains(&expected.to_lowercase())
                } else {
                    content.contains(expected.as_str())
                }
            } else {
                true
            }
        }
        "regex" => {
            if let Some(pattern) = &eval.pattern {
                regex::Regex::new(pattern)
                    .map(|re| re.is_match(content))
                    .unwrap_or(false)
            } else {
                true
            }
        }
        "exact" => eval.expected.as_deref() == Some(content),
        _ => true,
    }
}
