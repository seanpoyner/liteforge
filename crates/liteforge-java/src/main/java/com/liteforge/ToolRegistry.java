package com.liteforge;

import java.util.List;

/**
 * Registry of {@link Tool}s available to an agent or {@link ToolExecutor}.
 *
 * <p>Tools are stored behind the JNI boundary; registered Java objects are
 * pinned with a JNI global reference so they remain GC-reachable for the
 * lifetime of the registry. Call {@link #close()} (or use try-with-resources)
 * to release the handle.
 *
 * <pre>{@code
 * try (ToolRegistry registry = new ToolRegistry()) {
 *     registry.register(new WeatherTool());
 *     registry.register(new ClockTool());
 *     // hand `registry` to an Agent or ToolExecutor
 * }
 * }</pre>
 */
public final class ToolRegistry implements AutoCloseable {
    static {
        NativeLibrary.ensureLoaded();
    }

    private final long handle;
    private volatile boolean closed = false;

    public ToolRegistry() {
        this.handle = nativeCreate();
        if (this.handle == 0) {
            throw new RuntimeException("Failed to create ToolRegistry");
        }
    }

    /** Register (or replace) a tool. */
    public void register(Tool tool) {
        ensureOpen();
        if (tool == null) {
            throw new NullPointerException("tool");
        }
        nativeRegister(handle, tool);
    }

    /** Remove a tool by name. Returns {@code true} if it was present. */
    public boolean unregister(String name) {
        ensureOpen();
        return nativeUnregister(handle, name);
    }

    public boolean contains(String name) {
        ensureOpen();
        return nativeContains(handle, name);
    }

    public int size() {
        ensureOpen();
        return nativeSize(handle);
    }

    public List<String> names() {
        ensureOpen();
        return nativeNames(handle);
    }

    /**
     * Tool definitions in the OpenAI function-calling format, encoded as a
     * JSON array string. Useful for passing directly into a chat-completion
     * request.
     */
    public String definitionsJson() {
        ensureOpen();
        return nativeDefinitionsJson(handle);
    }

    /** Package-private: used by {@link ToolExecutor} and agents. */
    long handle() {
        ensureOpen();
        return handle;
    }

    private void ensureOpen() {
        if (closed) {
            throw new IllegalStateException("ToolRegistry is closed");
        }
    }

    @Override
    public void close() {
        if (!closed) {
            closed = true;
            nativeDestroy(handle);
        }
    }

    private static native long nativeCreate();
    private static native void nativeDestroy(long handle);
    private static native void nativeRegister(long handle, Tool tool);
    private static native boolean nativeUnregister(long handle, String name);
    private static native boolean nativeContains(long handle, String name);
    private static native int nativeSize(long handle);
    private static native List<String> nativeNames(long handle);
    private static native String nativeDefinitionsJson(long handle);
}
