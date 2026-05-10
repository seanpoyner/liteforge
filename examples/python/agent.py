#!/usr/bin/env python3
"""
Agent example - configuration, memory, and tool integration.

Run with: python examples/python/agent.py

This example demonstrates:
- AgentConfig for configuring agents
- AgentMemory for short-term, long-term, and working memory
- Tool registration with agents

Note: This example demonstrates the agent structure without making
actual LLM API calls, so it works without an API key.
"""

from liteforge import (
    AgentConfig,
    AgentMemory,
    create_tool,
    ToolRegistry,
)


def calculator(args: dict) -> dict:
    """Basic math operations tool."""
    op = args.get("operation", "add")
    a = args.get("a", 0)
    b = args.get("b", 0)

    if op == "add":
        result = a + b
    elif op == "subtract":
        result = a - b
    elif op == "multiply":
        result = a * b
    elif op == "divide":
        if b == 0:
            raise ValueError("Division by zero")
        result = a / b
    else:
        raise ValueError(f"Unknown operation: {op}")

    return {"result": result}


def get_weather(args: dict) -> dict:
    """Get weather for a location."""
    location = args.get("location", "unknown")
    unit = args.get("unit", "celsius")
    temp = 22 if unit == "celsius" else 72
    return {
        "location": location,
        "temperature": temp,
        "unit": unit,
        "conditions": "sunny",
    }


def main():
    print("=== LiteForge Agent Example (Python) ===\n")

    # 1. Create an agent configuration
    print("1. Creating agent configuration...")
    config = (
        AgentConfig("assistant-agent")
        .with_system_prompt("You are a helpful assistant with access to tools.")
        .with_model("gpt-4")
        .with_max_steps(5)
        .with_temperature(0.7)
        .with_tool("calculator")
        .with_tool("get_weather")
    )

    print(f"   Name: {config.name}")
    print(f"   System prompt: {config.system_prompt}")
    print(f"   Model: {config.model}")
    print(f"   Max steps: {config.max_steps}")
    print(f"   Tools: {config.tools}")

    # 2. Set up agent memory
    print("\n2. Working with agent memory...")
    memory = AgentMemory()

    # Short-term memory (conversation history)
    print("   Adding messages to short-term memory...")
    memory.add_message("user", "What's the weather in Paris?")
    memory.add_message("assistant", "Let me check that for you.")
    memory.add_message("user", "Thanks!")
    print(f"   Message count: {memory.message_count()}")

    # View messages
    messages = memory.messages()
    print("   Messages in memory:")
    for i, msg in enumerate(messages):
        print(f"     [{i}] {msg['role']}: {msg['content']}")

    # Long-term memory (persistent facts)
    print("\n   Storing facts in long-term memory...")
    memory.remember("user_name", "Alice")
    memory.remember("preferred_units", "celsius")
    memory.remember("timezone", "Europe/Paris")

    print(f"   Recalled user_name: {memory.recall('user_name')}")
    print(f"   Recalled preferred_units: {memory.recall('preferred_units')}")
    print(f"   Recalled timezone: {memory.recall('timezone')}")

    # Forget a fact
    memory.forget("timezone")
    print(f"   After forgetting timezone: {memory.recall('timezone')}")

    # 3. Register tools for the agent
    print("\n3. Setting up tools for the agent...")
    registry = ToolRegistry()

    calc_tool = create_tool(
        name="calculator",
        description="Perform basic math operations",
        parameters={
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                },
                "a": {"type": "number"},
                "b": {"type": "number"},
            },
            "required": ["operation", "a", "b"],
        },
        func=calculator,
        requires_confirmation=False,
    )

    weather_tool = create_tool(
        name="get_weather",
        description="Get current weather for a location",
        parameters={
            "type": "object",
            "properties": {
                "location": {"type": "string", "description": "City name"},
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "Temperature unit",
                },
            },
            "required": ["location"],
        },
        func=get_weather,
        requires_confirmation=False,
    )

    registry.register(calc_tool)
    registry.register(weather_tool)

    print(f"   Registered tools: {registry.names()}")
    print(f"   Contains 'calculator': {registry.contains('calculator')}")

    # 4. Demonstrate agent workflow simulation
    print("\n4. Simulating agent workflow...")
    print("   (In real usage, the agent would interact with an LLM)")

    # Simulate a conversation flow
    workflow_memory = AgentMemory()

    # User asks a question
    workflow_memory.add_message("user", "What is 15 multiplied by 7?")
    print("   User: What is 15 multiplied by 7?")

    # Agent would call tool (simulated)
    result = calculator({"operation": "multiply", "a": 15, "b": 7})
    print(f"   [Tool call] calculator -> {result}")

    # Agent responds
    workflow_memory.add_message("assistant", f"15 multiplied by 7 equals {result['result']}.")
    print(f"   Assistant: 15 multiplied by 7 equals {result['result']}.")

    # User asks follow-up
    workflow_memory.add_message("user", "What's the weather in Tokyo?")
    print("   User: What's the weather in Tokyo?")

    # Agent calls another tool
    weather_result = get_weather({"location": "Tokyo", "unit": "celsius"})
    print(f"   [Tool call] get_weather -> {weather_result}")

    # Agent responds
    response = f"The weather in {weather_result['location']} is {weather_result['temperature']}°{weather_result['unit'][0].upper()} and {weather_result['conditions']}."
    workflow_memory.add_message("assistant", response)
    print(f"   Assistant: {response}")

    # 5. Check final memory state
    print("\n5. Final memory state...")
    print(f"   Total messages: {workflow_memory.message_count()}")
    print("   Conversation history:")
    for msg in workflow_memory.messages():
        print(f"     {msg['role']}: {msg['content'][:50]}...")

    # 6. Clear memory
    print("\n6. Clearing short-term memory...")
    print(f"   Before clear: {workflow_memory.message_count()} messages")
    workflow_memory.clear_short_term()
    print(f"   After clear: {workflow_memory.message_count()} messages")

    print("\n=== Example Complete ===")


if __name__ == "__main__":
    main()
