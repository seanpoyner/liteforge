package com.liteforge;

/**
 * Executes {@link Tool}s from a {@link ToolRegistry} with JSON-schema
 * validation and execution-time tracking.
 *
 * <p>The executor borrows the registry; closing the executor does not close
 * the registry, and the registry must outlive any executor built from it.
 *
 * <pre>{@code
 * try (ToolRegistry registry = new ToolRegistry();
 *      ToolExecutor exec = new ToolExecutor(registry)) {
 *     registry.register(new WeatherTool());
 *     ToolResult r = exec.execute("get_weather", "{\"city\":\"Paris\"}");
 *     if (r.isSuccess()) {
 *         System.out.println(r.getResultJson());
 *     }
 * }
 * }</pre>
 */
public final class ToolExecutor implements AutoCloseable {
    static {
        NativeLibrary.ensureLoaded();
    }

    private final long handle;
    private volatile boolean closed = false;

    public ToolExecutor(ToolRegistry registry) {
        if (registry == null) {
            throw new NullPointerException("registry");
        }
        this.handle = nativeCreate(registry.handle());
        if (this.handle == 0) {
            throw new RuntimeException("Failed to create ToolExecutor");
        }
    }

    /** Run a tool and return the result. */
    public ToolResult execute(String name, String argsJson) {
        ensureOpen();
        return nativeExecute(handle, name, argsJson);
    }

    /**
     * Run a tool using the given call ID. Use this when responding to an
     * LLM-initiated tool call — pass the call's ID so the result can be
     * correlated back to the conversation.
     */
    public ToolResult executeWithId(String callId, String name, String argsJson) {
        ensureOpen();
        return nativeExecuteWithId(handle, callId, name, argsJson);
    }

    private void ensureOpen() {
        if (closed) {
            throw new IllegalStateException("ToolExecutor is closed");
        }
    }

    @Override
    public void close() {
        if (!closed) {
            closed = true;
            nativeDestroy(handle);
        }
    }

    private static native long nativeCreate(long registryHandle);
    private static native void nativeDestroy(long handle);
    private static native ToolResult nativeExecute(long handle, String name, String argsJson);
    private static native ToolResult nativeExecuteWithId(long handle, String callId, String name, String argsJson);
}
