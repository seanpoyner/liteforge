use crate::error::CliError;
use crate::input::resolve_prompt;
use crate::theme;
use regex::Regex;
use serde::Serialize;

#[derive(clap::Args)]
pub struct GuardrailsArgs {
    #[command(subcommand)]
    pub command: GuardrailsCommand,
}

#[derive(clap::Subcommand)]
pub enum GuardrailsCommand {
    /// Check text for potential issues
    Check {
        /// Text to check (reads from stdin if omitted)
        text: Option<String>,

        /// Read from file
        #[arg(short, long)]
        file: Option<String>,

        /// Read from stdin
        #[arg(long)]
        stdin: bool,

        /// Check for PII
        #[arg(long)]
        pii: bool,

        /// Check for injection attempts
        #[arg(long)]
        injection: bool,

        /// Run all checks
        #[arg(long)]
        all: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct CheckResult {
    clean: bool,
    issues: Vec<Issue>,
}

#[derive(Debug, Serialize)]
struct Issue {
    category: String,
    severity: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    match_text: Option<String>,
}

pub async fn execute(args: GuardrailsArgs) -> Result<(), CliError> {
    match args.command {
        GuardrailsCommand::Check {
            text,
            file,
            stdin,
            pii,
            injection,
            all,
            json,
        } => {
            let input = resolve_prompt(text, file, stdin)?;
            let check_pii = pii || all || !injection;
            let check_injection = injection || all || !pii;

            let mut issues = Vec::new();

            if check_pii {
                issues.extend(check_pii_patterns(&input));
            }

            if check_injection {
                issues.extend(check_injection_patterns(&input));
            }

            let result = CheckResult {
                clean: issues.is_empty(),
                issues,
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            } else {
                print_result(&result);
            }

            Ok(())
        }
    }
}

fn check_pii_patterns(text: &str) -> Vec<Issue> {
    let mut issues = Vec::new();

    // Email addresses
    let email_re = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
    for m in email_re.find_iter(text) {
        issues.push(Issue {
            category: "PII".to_string(),
            severity: "medium".to_string(),
            description: "Email address detected".to_string(),
            match_text: Some(mask_middle(m.as_str())),
        });
    }

    // Phone numbers (various formats)
    let phone_re =
        Regex::new(r"\b(?:\+?1[-.\s]?)?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}\b").unwrap();
    for m in phone_re.find_iter(text) {
        issues.push(Issue {
            category: "PII".to_string(),
            severity: "medium".to_string(),
            description: "Phone number detected".to_string(),
            match_text: Some(mask_middle(m.as_str())),
        });
    }

    // SSN
    let ssn_re = Regex::new(r"\b[0-9]{3}-[0-9]{2}-[0-9]{4}\b").unwrap();
    for m in ssn_re.find_iter(text) {
        issues.push(Issue {
            category: "PII".to_string(),
            severity: "high".to_string(),
            description: "SSN pattern detected".to_string(),
            match_text: Some(mask_middle(m.as_str())),
        });
    }

    // Credit card numbers
    let cc_re = Regex::new(r"\b[0-9]{4}[-\s]?[0-9]{4}[-\s]?[0-9]{4}[-\s]?[0-9]{4}\b").unwrap();
    for m in cc_re.find_iter(text) {
        issues.push(Issue {
            category: "PII".to_string(),
            severity: "high".to_string(),
            description: "Credit card number pattern detected".to_string(),
            match_text: Some(mask_middle(m.as_str())),
        });
    }

    // IP addresses
    let ip_re = Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").unwrap();
    for m in ip_re.find_iter(text) {
        issues.push(Issue {
            category: "PII".to_string(),
            severity: "low".to_string(),
            description: "IP address detected".to_string(),
            match_text: Some(m.as_str().to_string()),
        });
    }

    issues
}

fn check_injection_patterns(text: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    let lower = text.to_lowercase();

    // Prompt injection patterns
    let injection_patterns = [
        (
            "ignore previous instructions",
            "high",
            "Prompt override attempt",
        ),
        ("ignore all instructions", "high", "Prompt override attempt"),
        ("disregard previous", "high", "Prompt override attempt"),
        ("forget everything", "medium", "Memory manipulation attempt"),
        ("you are now", "medium", "Role manipulation attempt"),
        ("act as if", "medium", "Role manipulation attempt"),
        ("pretend you are", "medium", "Role manipulation attempt"),
        ("jailbreak", "high", "Jailbreak keyword"),
        ("dan mode", "high", "Jailbreak keyword"),
        ("developer mode", "medium", "Jailbreak keyword"),
        ("system prompt", "low", "System prompt reference"),
        (
            "reveal your instructions",
            "medium",
            "Instruction extraction attempt",
        ),
        (
            "show me your prompt",
            "medium",
            "Instruction extraction attempt",
        ),
    ];

    for (pattern, severity, description) in injection_patterns {
        if lower.contains(pattern) {
            issues.push(Issue {
                category: "Injection".to_string(),
                severity: severity.to_string(),
                description: description.to_string(),
                match_text: Some(pattern.to_string()),
            });
        }
    }

    // Code injection patterns
    let code_patterns = [
        (r"<script", "high", "Script tag detected"),
        (r"javascript:", "high", "JavaScript URI detected"),
        (r"eval\s*\(", "medium", "Eval function call"),
        (r"exec\s*\(", "medium", "Exec function call"),
        (r"__import__", "medium", "Python import detected"),
    ];

    for (pattern, severity, description) in code_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(&lower) {
                issues.push(Issue {
                    category: "Injection".to_string(),
                    severity: severity.to_string(),
                    description: description.to_string(),
                    match_text: None,
                });
            }
        }
    }

    issues
}

fn print_result(result: &CheckResult) {
    if result.clean {
        println!("{}", theme::success("✓ No issues detected"));
        return;
    }

    println!(
        "{}",
        theme::warning(&format!("⚠ Found {} issue(s):", result.issues.len()))
    );
    println!();

    for issue in &result.issues {
        let severity_str = match issue.severity.as_str() {
            "high" => theme::error_text(&format!("[{}]", issue.severity.to_uppercase())),
            "medium" => theme::warning(&format!("[{}]", issue.severity.to_uppercase())),
            _ => theme::dimmed(&format!("[{}]", issue.severity.to_uppercase())),
        };

        println!(
            "  {} {} - {}",
            severity_str,
            theme::label(&issue.category),
            issue.description
        );

        if let Some(matched) = &issue.match_text {
            println!("      Match: {}", theme::dimmed(matched));
        }
    }
}

fn mask_middle(s: &str) -> String {
    let len = s.len();
    if len <= 4 {
        return "*".repeat(len);
    }
    let visible = 2;
    format!(
        "{}{}{}",
        &s[..visible],
        "*".repeat(len - visible * 2),
        &s[len - visible..]
    )
}
