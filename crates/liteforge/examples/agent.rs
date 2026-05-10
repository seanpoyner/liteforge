//! Agent example - configuration, memory, and tool integration.
//!
//! Run with: cargo run --example agent
//!
//! This example demonstrates:
//! - AgentConfig for configuring agents
//! - AgentMemory for short-term, long-term, and working memory
//! - AgentContext for managing agent state
//! - Tool registration with agents
//!
//! Note: This example demonstrates the agent structure without making
//! actual LLM API calls, so it works without an API key.

use serde_json::json;
use liteforge::agents::{AgentConfig, AgentContext, AgentMemory, AgentState};
use liteforge::tools::{FnTool, ToolRegistry};
use liteforge::types::Message;

fn main() {
    println!("=== LiteForge Agent Example ===\n");

    // 1. Create an agent configuration
    println!("1. Creating agent configuration...");
    let config = AgentConfig::new("assistant-agent")
        .with_system_prompt("You are a helpful assistant with access to tools.")
        .with_model("gpt-4")
        .with_max_steps(5)
        .with_temperature(0.7)
        .with_tool("calculator")
        .with_tool("get_weather");

    println!("   Name: {}", config.name);
    println!("   System prompt: {:?}", config.system_prompt);
    println!("   Model: {:?}", config.model);
    println!("   Max steps: {}", config.max_steps);
    println!("   Tools: {:?}", config.tools);

    // 2. Set up agent memory
    println!("\n2. Working with agent memory...");
    let mut memory = AgentMemory::new();

    // Short-term memory (conversation history)
    println!("   Adding messages to short-term memory...");
    memory.add_message(Message::user("What's the weather in Paris?"));
    memory.add_message(Message::assistant("Let me check that for you."));
    memory.add_message(Message::user("Thanks!"));
    println!("   Message count: {}", memory.message_count());

    // Long-term memory (persistent facts)
    println!("   Storing facts in long-term memory...");
    memory.remember("user_name", json!("Alice"));
    memory.remember("preferred_units", json!("celsius"));
    memory.remember("timezone", json!("Europe/Paris"));

    println!("   Recalled user_name: {:?}", memory.recall("user_name"));
    println!(
        "   Recalled preferred_units: {:?}",
        memory.recall("preferred_units")
    );

    // Working memory (scratch space for current task)
    println!("   Using working memory for current task...");
    memory.set_working("current_task", json!("weather_lookup"));
    memory.set_working("task_progress", json!({"step": 1, "status": "in_progress"}));
    println!(
        "   Working memory task: {:?}",
        memory.get_working("current_task")
    );

    // 3. Create agent context
    println!("\n3. Creating agent context...");
    let mut context = AgentContext::new("agent-001")
        .with_max_steps(10)
        .with_system_prompt("You are a helpful assistant.")
        .with_correlation_id("request-12345")
        .with_metadata("session_id", json!("sess_abc123"));

    println!("   Agent ID: {}", context.agent_id);
    println!("   State: {}", context.state);
    println!("   Can continue: {}", context.can_continue());

    // Simulate agent execution steps
    println!("\n4. Simulating agent execution flow...");
    context.state = AgentState::Thinking;
    println!(
        "   Step {}: State = {}",
        context.current_step, context.state
    );

    context.increment_step();
    context.state = AgentState::ExecutingTool;
    println!(
        "   Step {}: State = {}",
        context.current_step, context.state
    );

    context.increment_step();
    context.state = AgentState::Thinking;
    println!(
        "   Step {}: State = {}",
        context.current_step, context.state
    );

    context.increment_step();
    context.state = AgentState::Completed;
    println!(
        "   Step {}: State = {}",
        context.current_step, context.state
    );
    println!("   Can continue: {}", context.can_continue());

    // 5. Register tools for the agent
    println!("\n5. Setting up tools for the agent...");
    let mut registry = ToolRegistry::new();

    let calculator = FnTool::new(
        "calculator",
        "Perform basic math operations",
        json!({
            "type": "object",
            "properties": {
                "operation": { "type": "string", "enum": ["add", "subtract", "multiply", "divide"] },
                "a": { "type": "number" },
                "b": { "type": "number" }
            },
            "required": ["operation", "a", "b"]
        }),
        |args| {
            let op = args["operation"].as_str().unwrap_or("add");
            let a = args["a"].as_f64().unwrap_or(0.0);
            let b = args["b"].as_f64().unwrap_or(0.0);
            let result = match op {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" if b != 0.0 => a / b,
                "divide" => return Err("Division by zero".to_string()),
                _ => return Err(format!("Unknown operation: {}", op)),
            };
            Ok(json!({ "result": result }))
        },
    );

    let weather = FnTool::new(
        "get_weather",
        "Get current weather for a location",
        json!({
            "type": "object",
            "properties": {
                "location": { "type": "string" },
                "unit": { "type": "string", "enum": ["celsius", "fahrenheit"] }
            },
            "required": ["location"]
        }),
        |args| {
            let location = args["location"].as_str().unwrap_or("unknown");
            let unit = args["unit"].as_str().unwrap_or("celsius");
            Ok(json!({
                "location": location,
                "temperature": if unit == "celsius" { 22 } else { 72 },
                "unit": unit,
                "conditions": "sunny"
            }))
        },
    );

    registry.register(Box::new(calculator));
    registry.register(Box::new(weather));

    println!("   Registered tools: {:?}", registry.names());
    println!(
        "   Tool definitions ready for LLM: {}",
        registry.definitions().len()
    );

    // 6. Get messages for LLM request
    println!("\n6. Preparing messages for LLM request...");
    let mut ctx = AgentContext::new("demo-agent")
        .with_system_prompt("You are a helpful assistant with calculator and weather tools.");

    ctx.memory.add_message(Message::user("What's 15 * 7?"));

    let messages = ctx.get_messages();
    println!("   Messages prepared for API:");
    for (i, msg) in messages.iter().enumerate() {
        println!("     [{}] {}: {:?}", i, msg.role, msg.content);
    }

    // 7. Demonstrate context reset
    println!("\n7. Resetting agent context...");
    println!(
        "   Before reset: {} messages, step {}",
        ctx.memory.message_count(),
        ctx.current_step
    );
    ctx.reset();
    println!(
        "   After reset: {} messages, step {}",
        ctx.memory.message_count(),
        ctx.current_step
    );

    println!("\n=== Example Complete ===");
}
