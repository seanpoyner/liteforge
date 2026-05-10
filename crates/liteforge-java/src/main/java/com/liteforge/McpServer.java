package com.liteforge;

/**
 * A connection to an MCP (Model Context Protocol) server.
 *
 * <p>Supports all three MCP transports — stdio, HTTP, SSE — dispatched by the
 * {@link McpServerConfig}. All operations use the {@link ForgeClient}'s shared
 * tokio runtime.
 *
 * <pre>{@code
 * try (ForgeClient client = new ForgeClient();
 *      ToolRegistry registry = new ToolRegistry();
 *      McpServer server = new McpServer(client,
 *          McpServerConfig.stdio("fs", "npx")
 *              .withArg("-y")
 *              .withArg("@modelcontextprotocol/server-filesystem")
 *              .withArg("/tmp"))) {
 *     server.connect();
 *     int added = server.registerTools(registry);
 *     System.out.printf("Registered %d MCP tools%n", added);
 *
 *     // Now an agent with this registry can call filesystem tools.
 * }
 * }</pre>
 */
public final class McpServer implements AutoCloseable {
    static {
        NativeLibrary.ensureLoaded();
    }

    private final ForgeClient client;
    private final McpServerConfig config;
    private final long handle;
    private volatile boolean closed = false;

    public McpServer(ForgeClient client, McpServerConfig config) {
        if (client == null) throw new NullPointerException("client");
        if (config == null) throw new NullPointerException("config");
        this.client = client;
        this.config = config;
        this.handle = nativeCreate(client.handle(), config.toJson());
        if (this.handle == 0) {
            throw new RuntimeException("Failed to create McpServer");
        }
    }

    public String getName() {
        return config.getName();
    }

    public McpServerConfig getConfig() {
        return config;
    }

    /** Establish the MCP connection (launches subprocess / opens HTTP). */
    public void connect() {
        ensureOpen();
        nativeConnect(handle);
    }

    /** Tear down the MCP connection. */
    public void disconnect() {
        ensureOpen();
        nativeDisconnect(handle);
    }

    public boolean isConnected() {
        ensureOpen();
        return nativeIsConnected(handle);
    }

    /** Returns tools as a JSON array string (MCP tool definitions). */
    public String listToolsJson() {
        ensureOpen();
        return nativeListToolsJson(handle);
    }

    /**
     * Call an MCP tool by name. Arguments are a JSON object string; result is
     * a JSON {@code CallToolResult} — see the MCP spec for the shape.
     */
    public String callTool(String toolName, String argsJson) {
        ensureOpen();
        return nativeCallToolJson(handle, toolName, argsJson);
    }

    /**
     * Register every MCP tool exposed by this server into {@code registry} so
     * an agent sharing that registry can invoke them. Returns the number of
     * tools added. The server must already be {@link #connect() connected}.
     */
    public int registerTools(ToolRegistry registry) {
        ensureOpen();
        if (registry == null) throw new NullPointerException("registry");
        return nativeRegisterTools(handle, registry.handle());
    }

    private void ensureOpen() {
        if (closed) {
            throw new IllegalStateException("McpServer is closed");
        }
    }

    @Override
    public void close() {
        if (!closed) {
            closed = true;
            try {
                if (nativeIsConnected(handle)) {
                    nativeDisconnect(handle);
                }
            } catch (Throwable ignored) {
                // best-effort cleanup
            }
            nativeDestroy(handle);
        }
    }

    private static native long nativeCreate(long clientHandle, String configJson);
    private static native void nativeDestroy(long handle);
    private static native void nativeConnect(long handle);
    private static native void nativeDisconnect(long handle);
    private static native boolean nativeIsConnected(long handle);
    private static native String nativeListToolsJson(long handle);
    private static native String nativeCallToolJson(long handle, String toolName, String argsJson);
    private static native int nativeRegisterTools(long handle, long registryHandle);
}
