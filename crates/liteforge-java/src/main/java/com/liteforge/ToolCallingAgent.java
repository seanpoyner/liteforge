package com.liteforge;

import java.util.concurrent.CompletableFuture;

/**
 * An LLM-powered agent that can call {@link Tool}s to accomplish tasks.
 *
 * <p>The agent borrows both the {@link ForgeClient} (for LLM calls) and the
 * {@link ToolRegistry} (for tools). Both must remain open for the agent's
 * lifetime. The tool snapshot is taken at construction time — tools added to
 * the registry later will NOT be seen by this agent.
 *
 * <pre>{@code
 * try (ForgeClient client = new ForgeClient();
 *      ToolRegistry tools = new ToolRegistry()) {
 *     tools.register(new WeatherTool());
 *
 *     AgentConfig cfg = new AgentConfig("weather-bot")
 *         .withSystemPrompt("You help users check the weather.")
 *         .withMaxSteps(5);
 *
 *     try (ToolCallingAgent agent = new ToolCallingAgent(client, tools).withConfig(cfg)) {
 *         String answer = agent.run("What's the weather in Paris?");
 *         System.out.println(answer);
 *     }
 * }
 * }</pre>
 */
public final class ToolCallingAgent implements AutoCloseable {
    static {
        NativeLibrary.ensureLoaded();
    }

    private final ForgeClient client;
    private final ToolRegistry registry;
    private long handle;
    private AgentConfig config;
    private volatile boolean closed = false;

    public ToolCallingAgent(ForgeClient client, ToolRegistry registry) {
        if (client == null) throw new NullPointerException("client");
        if (registry == null) throw new NullPointerException("registry");
        this.client = client;
        this.registry = registry;
        this.config = new AgentConfig("agent");
        this.handle = build(this.config);
    }

    /** Replace the configuration. Tools registered later will NOT be seen. */
    public ToolCallingAgent withConfig(AgentConfig config) {
        ensureOpen();
        if (config == null) throw new NullPointerException("config");
        // Tear down and rebuild so the snapshot reflects the new config.
        nativeDestroy(handle);
        this.config = config;
        this.handle = build(config);
        return this;
    }

    /** Convenience for {@code withConfig(new AgentConfig(name).withSystemPrompt(prompt))}. */
    public ToolCallingAgent withSystemPrompt(String prompt) {
        return withConfig(new AgentConfig(config.getName()).withSystemPrompt(prompt)
                .withMaxSteps(config.getMaxSteps()));
    }

    /**
     * Run the agent against a user message. Blocks until the agent finishes
     * or throws. Uses the {@link ForgeClient}'s tokio runtime.
     */
    public String run(String input) {
        ensureOpen();
        if (input == null) throw new NullPointerException("input");
        return nativeRun(client.handle(), handle, input);
    }

    /**
     * Non-blocking run. Completes the returned future with the final agent
     * response, or fails it with the error message.
     */
    public CompletableFuture<String> runAsync(String input) {
        ensureOpen();
        if (input == null) throw new NullPointerException("input");
        CompletableFuture<String> future = new CompletableFuture<>();
        nativeRunAsync(client.handle(), handle, input, new AgentCallback() {
            @Override public void onSuccess(String response) { future.complete(response); }
            @Override public void onError(String error) {
                future.completeExceptionally(new RuntimeException(error));
            }
        });
        return future;
    }

    /** Clear conversation history and reset the step counter. */
    public void reset() {
        ensureOpen();
        nativeReset(client.handle(), handle);
    }

    public AgentConfig config() {
        return config;
    }

    private long build(AgentConfig cfg) {
        long h = nativeCreate(client.handle(), registry.handle(), cfg.toJson());
        if (h == 0) {
            throw new RuntimeException("Failed to create ToolCallingAgent");
        }
        return h;
    }

    private void ensureOpen() {
        if (closed) {
            throw new IllegalStateException("ToolCallingAgent is closed");
        }
    }

    @Override
    public void close() {
        if (!closed) {
            closed = true;
            nativeDestroy(handle);
        }
    }

    private static native long nativeCreate(long clientHandle, long registryHandle, String configJson);
    private static native void nativeDestroy(long handle);
    private static native String nativeRun(long clientHandle, long agentHandle, String input);
    private static native void nativeRunAsync(long clientHandle, long agentHandle, String input, AgentCallback callback);
    private static native void nativeReset(long clientHandle, long agentHandle);
}
