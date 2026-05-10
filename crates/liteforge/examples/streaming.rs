//! Streaming completion example.
//!
//! Run with: cargo run --example streaming

use futures::StreamExt;
use liteforge::{AsyncForgeClient, Message};

#[tokio::main]
async fn main() {
    // Create async client
    let client = AsyncForgeClient::new();

    println!("Using model: {}", client.model());

    // Create a streaming completion
    let messages = vec![Message::user("Tell me a short story about a robot.")];

    match client.complete_stream(messages).await {
        Ok(mut stream) => {
            print!("\nResponse: ");
            while let Some(result) = stream.next().await {
                match result {
                    Ok(chunk) => {
                        if let Some(content) = chunk.content() {
                            print!("{}", content);
                        }
                    }
                    Err(e) => {
                        eprintln!("\nStream error: {}", e);
                        break;
                    }
                }
            }
            println!();
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
