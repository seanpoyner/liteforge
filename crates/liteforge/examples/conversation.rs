//! Conversation example - managed conversations with context windowing.
//!
//! Run with: cargo run --example conversation -p liteforge
//!
//! This example demonstrates:
//! - ManagedConversation for tracking messages
//! - ConversationConfig for configuring behavior
//! - CompactingConversation for automatic summarization
//! - Token estimation and context management
//!
//! Note: Uses local conversation management with no external services,
//! no API keys required.

use liteforge::conversation::{
    CompactingConversation, ConversationConfig, ManagedConversation, SummarizationStrategy,
};

fn main() {
    println!("=== LiteForge Conversation Example ===\n");

    // 1. Basic ManagedConversation
    println!("1. Basic ManagedConversation...");

    let mut conv = ManagedConversation::new();
    conv.set_system("You are a helpful coding assistant.");
    conv.add_user_message("Hello! Can you help me with Rust?");
    conv.add_assistant_message(
        "Of course! I'd be happy to help with Rust. What would you like to know?",
    );
    conv.add_user_message("How do I create a vector?");
    conv.add_assistant_message("You can create a vector using `Vec::new()` or the `vec![]` macro.");

    println!("   Messages: {}", conv.len());
    println!("   Estimated tokens: ~{}", conv.estimated_tokens());
    println!("   Has system message: {}", conv.system_message().is_some());

    // 2. Access messages
    println!("\n2. Accessing messages...");

    let all_messages = conv.messages();
    println!(
        "   Total messages (including system): {}",
        all_messages.len()
    );

    let last_msg = conv.last_message();
    if let Some(msg) = last_msg {
        let content = msg.content.as_deref().unwrap_or("");
        println!(
            "   Last message: \"{}...\"",
            &content[..content.len().min(40)]
        );
    }

    // Get last N messages
    let recent = conv.last_n_messages(2);
    println!("   Last 2 messages:");
    for msg in &recent {
        println!(
            "     - {}: {}",
            msg.role,
            msg.content.as_deref().unwrap_or("")
        );
    }

    // 3. Search and filter messages
    println!("\n3. Searching messages...");

    let found = conv.find_messages("rust");
    println!("   Messages containing 'rust': {}", found.len());
    for tracked in &found {
        let content = tracked.message.content.as_deref().unwrap_or("");
        let preview = &content[..content.len().min(50)];
        println!("     - \"{}...\"", preview);
    }

    let user_msgs = conv.messages_by_role("user");
    println!("   User messages: {}", user_msgs.len());

    let assistant_msgs = conv.messages_by_role("assistant");
    println!("   Assistant messages: {}", assistant_msgs.len());

    // 4. Conversation metadata
    println!("\n4. Managing metadata...");

    conv.set_metadata("session_id", "sess_abc123");
    conv.set_metadata("user_id", "user_456");
    conv.set_metadata("topic", "rust_programming");

    println!("   Session ID: {:?}", conv.get_metadata("session_id"));
    println!("   User ID: {:?}", conv.get_metadata("user_id"));
    println!("   Topic: {:?}", conv.get_metadata("topic"));
    println!("   All metadata: {:?}", conv.metadata());

    // 5. Export as text
    println!("\n5. Exporting conversation...");

    let text = conv.to_text();
    println!("   Text representation:");
    for line in text.lines().take(6) {
        println!("     {}", line);
    }
    println!("     ...");

    // 6. ConversationConfig
    println!("\n6. ConversationConfig options...");

    let config = ConversationConfig::builder()
        .max_tokens(8000)
        .target_tokens(4000)
        .preserve_recent(4)
        .preserve_system(true)
        .strategy(SummarizationStrategy::Summarize)
        .summarization_prompt("Summarize the key points of this conversation:")
        .build();

    println!("   Max tokens: {}", config.max_tokens);
    println!("   Target tokens: {}", config.target_tokens);
    println!("   Preserve recent: {}", config.preserve_recent);
    println!("   Preserve system: {}", config.preserve_system);
    println!("   Strategy: {:?}", config.strategy);

    // 7. SummarizationStrategy options
    println!("\n7. Available summarization strategies...");

    let strategies = [
        (
            SummarizationStrategy::KeepRecent,
            "Keep only recent messages, discard old ones",
        ),
        (
            SummarizationStrategy::Summarize,
            "Summarize older messages into system context",
        ),
        (
            SummarizationStrategy::ChunkedSummary,
            "Create multiple summary levels",
        ),
        (
            SummarizationStrategy::PreserveSystem,
            "Always preserve system messages",
        ),
    ];

    for (strategy, description) in &strategies {
        println!("   {:?}: {}", strategy, description);
    }

    // 8. CompactingConversation
    println!("\n8. CompactingConversation with auto-summarization...");

    // Create config with small limits to trigger compaction
    let compact_config = ConversationConfig::builder()
        .max_tokens(100)
        .target_tokens(50)
        .preserve_recent(2)
        .strategy(SummarizationStrategy::Summarize)
        .build();

    let mut compact_conv = CompactingConversation::new(compact_config);
    compact_conv.set_system("You are a helpful assistant.");

    // Add messages until compaction is needed
    println!("   Adding messages...");
    for i in 1..=10 {
        compact_conv.add_user_message(format!(
            "This is user message number {}. It has some content to use tokens.",
            i
        ));
        compact_conv.add_assistant_message(format!(
            "Response to message {}. Here is a helpful reply with information.",
            i
        ));
    }

    println!(
        "   Total messages added: {}",
        compact_conv.total_messages_added()
    );
    println!("   Current message count: {}", compact_conv.len());
    println!("   Estimated tokens: ~{}", compact_conv.estimated_tokens());
    println!("   Needs compaction: {}", compact_conv.needs_compaction());

    // 9. Perform compaction
    println!("\n9. Performing compaction...");

    if compact_conv.needs_compaction() {
        if let Some(result) = compact_conv.compact() {
            println!("   Compaction result:");
            println!("     Messages removed: {}", result.messages_removed);
            println!("     Tokens before: {}", result.tokens_before);
            println!("     Tokens after: {}", result.tokens_after);
            if let Some(summary) = &result.summary {
                println!(
                    "     Summary preview: \"{}...\"",
                    &summary[..summary.len().min(60)]
                );
            }
        }
    }

    println!("   After compaction:");
    println!("     Message count: {}", compact_conv.len());
    println!("     Compaction count: {}", compact_conv.compaction_count());
    println!("     Summaries stored: {}", compact_conv.summaries().len());

    // 10. Get messages for API
    println!("\n10. Getting messages for API call...");

    let api_messages = compact_conv.messages_for_api();
    println!("   Messages ready for API: {}", api_messages.len());
    for msg in &api_messages {
        let content = msg.content.as_deref().unwrap_or("");
        let preview = &content[..content.len().min(50)];
        println!("     - [{}] \"{}...\"", msg.role, preview);
    }

    // 11. Practical workflow
    println!("\n11. Practical conversation workflow...");

    fn simulate_conversation() {
        // Configure for a typical use case
        let config = ConversationConfig::builder()
            .max_tokens(4000)
            .target_tokens(2000)
            .preserve_recent(6)
            .strategy(SummarizationStrategy::Summarize)
            .build();

        let mut conv = CompactingConversation::with_system(
            "You are a helpful assistant that remembers our conversation.",
            config,
        );

        // Simulate a multi-turn conversation
        let exchanges = [
            ("What's Rust?", "Rust is a systems programming language focused on safety and performance."),
            ("What are its main features?", "Key features include memory safety without GC, zero-cost abstractions, and fearless concurrency."),
            ("How do I install it?", "Use rustup: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"),
            ("What's cargo?", "Cargo is Rust's build system and package manager."),
        ];

        for (user_msg, assistant_msg) in &exchanges {
            conv.add_user_message(*user_msg);
            conv.add_assistant_message(*assistant_msg);

            // Check if compaction needed after each exchange
            if conv.needs_compaction() {
                conv.compact();
            }
        }

        println!("   Conversation summary:");
        println!("     Exchanges: {}", exchanges.len());
        println!("     Messages retained: {}", conv.len());
        println!("     Compactions: {}", conv.compaction_count());
    }

    simulate_conversation();

    // 12. Clear operations
    println!("\n12. Clearing conversations...");

    // Clear but keep system message
    conv.clear(true);
    println!("   After clear(keep_system=true):");
    println!("     Messages: {}", conv.len());
    println!(
        "     System message preserved: {}",
        conv.system_message().is_some()
    );

    // Full clear
    conv.clear(false);
    println!("   After clear(keep_system=false):");
    println!("     Messages: {}", conv.len());
    println!("     System message: {}", conv.system_message().is_some());

    // 13. Message removal
    println!("\n13. Selective message removal...");

    let mut conv = ManagedConversation::new();
    for i in 1..=6 {
        conv.add_user_message(format!("Message {}", i));
    }

    println!("   Before removal: {} messages", conv.len());
    conv.remove_first(2);
    println!("   After remove_first(2): {} messages", conv.len());

    // First remaining message should be "Message 3"
    let remaining = conv.messages();
    if let Some(first) = remaining.first() {
        println!("   First remaining: {:?}", first.content);
    }

    // 14. TrackedMessage details
    println!("\n14. TrackedMessage metadata...");

    let mut conv = ManagedConversation::new();
    conv.add_user_message("Hello!");
    conv.add_assistant_message("Hi there!");

    for tracked in conv.tracked_messages() {
        println!("   Message ID: {}", tracked.id);
        println!("     Role: {}", tracked.message.role);
        println!("     Timestamp: {}", tracked.timestamp);
        println!("     Token estimate: {}", tracked.token_estimate);
    }

    println!("\n=== Example Complete ===");
}
