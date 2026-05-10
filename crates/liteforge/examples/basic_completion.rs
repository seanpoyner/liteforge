//! Basic completion example.
//!
//! Run with: cargo run --example basic_completion

use liteforge::{Message, ForgeClient};

fn main() {
    // Create client (reads LITEFORGE_API_KEY from environment)
    let client = ForgeClient::new();

    println!("Using model: {}", client.model());
    println!("Base URL: {}", client.base_url());

    // Create a simple completion
    let messages = vec![
        Message::system("You are a helpful assistant."),
        Message::user("What is the capital of France?"),
    ];

    match client.complete(messages) {
        Ok(response) => {
            println!("\nResponse:");
            println!("{}", response.content().unwrap_or("No content"));

            if let Some(usage) = &response.usage {
                println!("\nTokens used: {}", usage.total_tokens);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
