//! Tools example - defining, registering, and executing tools.
//!
//! Run with: cargo run --example tools
//!
//! This example demonstrates:
//! - Creating a custom tool by implementing the Tool trait
//! - Using FnTool for quick function-based tools
//! - ToolRegistry for managing multiple tools
//! - ToolExecutor for executing tool calls
//! - JSON schema validation

use serde_json::json;
use liteforge::tools::{FnTool, Tool, ToolExecutor, ToolRegistry, validate_json_schema};

// Define a custom tool by implementing the Tool trait
struct WeatherTool;

impl Tool for WeatherTool {
    fn name(&self) -> &str {
        "get_weather"
    }

    fn description(&self) -> &str {
        "Get the current weather for a location"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name, e.g. 'San Francisco'"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "Temperature unit"
                }
            },
            "required": ["location"]
        })
    }

    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, String> {
        let location = args["location"].as_str().unwrap_or("unknown");
        let unit = args["unit"].as_str().unwrap_or("fahrenheit");

        // Simulated weather data
        let temp = if unit == "celsius" { 22 } else { 72 };
        Ok(json!({
            "location": location,
            "temperature": temp,
            "unit": unit,
            "conditions": "sunny"
        }))
    }
}

fn main() {
    println!("=== LiteForge Tools Example ===\n");

    // 1. Create a custom tool
    println!("1. Creating custom WeatherTool...");
    let weather_tool = WeatherTool;
    println!("   Tool name: {}", weather_tool.name());
    println!("   Description: {}", weather_tool.description());

    // 2. Create function-based tools using FnTool
    println!("\n2. Creating FnTool-based calculator...");
    let calculator = FnTool::new(
        "calculator",
        "Perform basic math operations",
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"]
                },
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
                "divide" => {
                    if b == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    a / b
                }
                _ => return Err(format!("Unknown operation: {}", op)),
            };

            Ok(json!({ "result": result }))
        },
    );

    // 3. Register tools in a registry
    println!("\n3. Registering tools in ToolRegistry...");
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(weather_tool));
    registry.register(Box::new(calculator));

    println!("   Registered tools: {:?}", registry.names());
    println!("   Contains 'get_weather': {}", registry.contains("get_weather"));

    // 4. Get tool definitions (for LLM API)
    println!("\n4. Getting tool definitions for LLM...");
    let definitions = registry.definitions();
    println!("   {} tool definitions ready for API", definitions.len());

    // 5. Create executor and run tools
    println!("\n5. Executing tools via ToolExecutor...");
    let executor = ToolExecutor::new(registry);

    // Execute weather tool
    let weather_result = executor.execute(
        "get_weather",
        json!({"location": "San Francisco", "unit": "celsius"}),
    );
    println!("   Weather result: {:?}", weather_result);

    // Execute calculator
    let calc_result = executor.execute(
        "calculator",
        json!({"operation": "multiply", "a": 6, "b": 7}),
    );
    println!("   Calculator result: {:?}", calc_result);

    // 6. Schema validation
    println!("\n6. Validating arguments against schema...");
    let schema = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string", "minLength": 1 },
            "age": { "type": "integer", "minimum": 0 }
        },
        "required": ["name"]
    });

    // Valid input
    let valid_input = json!({"name": "Alice", "age": 30});
    match validate_json_schema(&valid_input, &schema) {
        Ok(()) => println!("   Valid input passed validation ✓"),
        Err(errors) => println!("   Validation errors: {:?}", errors),
    }

    // Invalid input (missing required field)
    let invalid_input = json!({"age": -5});
    match validate_json_schema(&invalid_input, &schema) {
        Ok(()) => println!("   Invalid input passed (unexpected)"),
        Err(errors) => {
            println!("   Invalid input caught {} error(s) ✓", errors.len());
            for e in &errors {
                println!("     - {}: {}", e.path, e.message);
            }
        }
    }

    println!("\n=== Example Complete ===");
}
