package com.liteforge;

/**
 * Result of a single tool execution.
 *
 * <p>{@link #getResultJson()} is only populated when {@link #isSuccess()} is
 * {@code true}; {@link #getError()} is only populated on failure. Parse the
 * JSON string with the library of your choice.
 */
public final class ToolResult {
    private final String toolCallId;
    private final String name;
    private final boolean success;
    private final String resultJson;
    private final String error;
    private final Long executionTimeMs;

    /** Called from native code via JNI. */
    ToolResult(
            String toolCallId,
            String name,
            boolean success,
            String resultJson,
            String error,
            Long executionTimeMs) {
        this.toolCallId = toolCallId;
        this.name = name;
        this.success = success;
        this.resultJson = resultJson;
        this.error = error;
        this.executionTimeMs = executionTimeMs;
    }

    public String getToolCallId() {
        return toolCallId;
    }

    public String getName() {
        return name;
    }

    public boolean isSuccess() {
        return success;
    }

    /** JSON-encoded result value, or {@code null} on failure. */
    public String getResultJson() {
        return resultJson;
    }

    /** Error message, or {@code null} on success. */
    public String getError() {
        return error;
    }

    /** Execution time in milliseconds, or {@code null} if not measured. */
    public Long getExecutionTimeMs() {
        return executionTimeMs;
    }

    @Override
    public String toString() {
        if (success) {
            return "ToolResult{name='" + name + "', success=true}";
        }
        return "ToolResult{name='" + name + "', success=false, error='" + error + "'}";
    }
}
