package com.liteforge;

/**
 * A callable tool that an LLM can invoke through function calling.
 *
 * <p>Implement this interface and register the instance with a
 * {@link ToolRegistry}. When an agent or {@link ToolExecutor} needs to run the
 * tool, {@link #execute(String)} is called from a native thread — so
 * implementations must be thread-safe.
 *
 * <p>Arguments and results cross the JNI boundary as JSON strings. Parse and
 * serialize them with the JSON library of your choice (Jackson, Gson,
 * org.json, etc.).
 *
 * <pre>{@code
 * Tool weather = new Tool() {
 *     public String name() { return "get_weather"; }
 *     public String description() { return "Get weather for a city"; }
 *     public String parametersSchemaJson() {
 *         return "{\"type\":\"object\",\"properties\":"
 *              + "{\"city\":{\"type\":\"string\"}},\"required\":[\"city\"]}";
 *     }
 *     public String execute(String argsJson) {
 *         // parse argsJson, do work, return JSON string
 *         return "{\"tempF\":72}";
 *     }
 * };
 * registry.register(weather);
 * }</pre>
 */
public interface Tool {
    /** Unique name of this tool. */
    String name();

    /** Human- and LLM-readable description. */
    String description();

    /**
     * JSON Schema describing the tool's arguments, returned as a JSON string.
     * Must be a JSON object with at least {@code type} and {@code properties}.
     */
    String parametersSchemaJson();

    /**
     * Run the tool. The native layer passes the LLM's arguments as a JSON
     * string and expects a JSON string back. Throw any exception on failure —
     * it will be surfaced to the caller as a tool error.
     */
    String execute(String argsJson) throws Exception;

    /**
     * Whether the host should require human confirmation before invoking this
     * tool. Default: {@code false}.
     */
    default boolean requiresConfirmation() {
        return false;
    }
}
