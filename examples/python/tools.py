#!/usr/bin/env python3
"""
Tools example - defining, registering, and executing tools.

Run with: python examples/python/tools.py

This example demonstrates:
- Creating tools using create_tool()
- ToolRegistry for managing multiple tools
- ToolExecutor for executing tool calls
- JSON schema validation
"""

from liteforge import (
    create_tool,
    ToolRegistry,
    ToolExecutor,
    validate_json_schema,
)


def get_weather(args: dict) -> dict:
    """Simulated weather lookup tool."""
    location = args.get("location", "unknown")
    unit = args.get("unit", "fahrenheit")
    temp = 22 if unit == "celsius" else 72
    return {
        "location": location,
        "temperature": temp,
        "unit": unit,
        "conditions": "sunny",
    }


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


def main():
    print("=== LiteForge Tools Example (Python) ===\n")

    # 1. Create tools using create_tool()
    print("1. Creating tools with create_tool()...")
    weather_tool = create_tool(
        name="get_weather",
        description="Get the current weather for a location",
        parameters={
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name, e.g. 'San Francisco'",
                },
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
    print(f"   Created: {weather_tool}")

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
    print(f"   Created: {calc_tool}")

    # 2. Register tools in a registry
    print("\n2. Registering tools in ToolRegistry...")
    registry = ToolRegistry()
    registry.register(weather_tool)
    registry.register(calc_tool)
    print(f"   Registry has {len(registry)} tools")
    print(f"   Contains 'get_weather': {registry.contains('get_weather')}")

    # 3. Create executor and run tools
    print("\n3. Executing tools via ToolExecutor...")
    executor = ToolExecutor(registry, validate_args=True)

    # Execute weather tool
    weather_result = executor.execute(
        "get_weather", {"location": "San Francisco", "unit": "celsius"}
    )
    print(f"   Weather result: success={weather_result.success}")
    print(f"   Weather data: {weather_result.result}")

    # Execute calculator
    calc_result = executor.execute(
        "calculator", {"operation": "multiply", "a": 6, "b": 7}
    )
    print(f"   Calculator result: success={calc_result.success}")
    print(f"   Calculator data: {calc_result.result}")

    # 4. Schema validation
    print("\n4. Validating arguments against schema...")
    schema = {
        "type": "object",
        "properties": {
            "name": {"type": "string", "minLength": 1},
            "age": {"type": "integer", "minimum": 0},
        },
        "required": ["name"],
    }

    # Valid input
    valid_input = {"name": "Alice", "age": 30}
    try:
        validate_json_schema(valid_input, schema)
        print("   Valid input passed validation ✓")
    except ValueError as e:
        print(f"   Validation error: {e}")

    # Invalid input (missing required field)
    invalid_input = {"age": -5}
    try:
        validate_json_schema(invalid_input, schema)
        print("   Invalid input passed (unexpected)")
    except ValueError as e:
        print(f"   Invalid input caught ✓")
        print(f"     Error: {e}")

    print("\n=== Example Complete ===")


if __name__ == "__main__":
    main()
