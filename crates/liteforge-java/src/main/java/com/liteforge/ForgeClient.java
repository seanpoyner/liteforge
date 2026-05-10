package com.liteforge;

import java.util.List;
import java.util.concurrent.CompletableFuture;

/**
 * Client for the LiteForge API.
 *
 * <p>Example usage:</p>
 * <pre>{@code
 * try (ForgeClient client = new ForgeClient()) {
 *     ChatCompletion response = client.complete(List.of(
 *         Message.user("Hello!")
 *     ));
 *     System.out.println(response.getContent());
 * }
 * }</pre>
 */
public final class ForgeClient implements AutoCloseable {
    private final long handle;
    private volatile boolean closed = false;

    static {
        NativeLibrary.ensureLoaded();
    }

    /**
     * Creates a new ForgeClient with default configuration.
     * Reads API key from LITEFORGE_API_KEY or OPENAI_API_KEY environment variable.
     */
    public ForgeClient() {
        this.handle = nativeCreate();
        if (this.handle == 0) {
            throw new RuntimeException("Failed to create ForgeClient");
        }
    }

    /**
     * Creates a new ForgeClient with the specified configuration.
     * Package-private: use {@link #builder()} instead.
     */
    ForgeClient(String apiKey, String baseUrl, String defaultModel, int timeoutSeconds) {
        this.handle = nativeCreateWithConfig(apiKey, baseUrl, defaultModel, timeoutSeconds);
        if (this.handle == 0) {
            throw new RuntimeException("Failed to create ForgeClient");
        }
    }

    /**
     * Creates a new builder for ForgeClient configuration.
     *
     * @return A new ForgeClientBuilder
     */
    public static ForgeClientBuilder builder() {
        return new ForgeClientBuilder();
    }

    /**
     * Gets the default model name.
     *
     * @return The default model name
     */
    public String getModel() {
        ensureOpen();
        return nativeGetModel(handle);
    }

    /**
     * Gets the base URL.
     *
     * @return The base URL
     */
    public String getBaseUrl() {
        ensureOpen();
        return nativeGetBaseUrl(handle);
    }

    /**
     * Completes a chat conversation using the default model.
     *
     * @param messages The conversation messages
     * @return The chat completion response
     */
    public ChatCompletion complete(List<Message> messages) {
        ensureOpen();
        return nativeComplete(handle, messages);
    }

    /**
     * Completes a chat conversation using the specified model.
     *
     * @param model    The model to use
     * @param messages The conversation messages
     * @return The chat completion response
     */
    public ChatCompletion complete(String model, List<Message> messages) {
        ensureOpen();
        return nativeCompleteWithModel(handle, model, messages);
    }

    /**
     * Asynchronously completes a chat conversation using the default model.
     *
     * @param messages The conversation messages
     * @return A CompletableFuture that will contain the chat completion response
     */
    public CompletableFuture<ChatCompletion> completeAsync(List<Message> messages) {
        ensureOpen();
        CompletableFuture<ChatCompletion> future = new CompletableFuture<>();

        nativeCompleteAsync(handle, messages, new CompletionCallback() {
            @Override
            public void onSuccess(ChatCompletion completion) {
                future.complete(completion);
            }

            @Override
            public void onError(String error) {
                future.completeExceptionally(new RuntimeException(error));
            }
        });

        return future;
    }

    /**
     * Asynchronously completes a chat conversation using the specified model.
     *
     * @param model    The model to use
     * @param messages The conversation messages
     * @return A CompletableFuture that will contain the chat completion response
     */
    public CompletableFuture<ChatCompletion> completeAsync(String model, List<Message> messages) {
        ensureOpen();
        CompletableFuture<ChatCompletion> future = new CompletableFuture<>();

        nativeCompleteAsyncWithModel(handle, model, messages, new CompletionCallback() {
            @Override
            public void onSuccess(ChatCompletion completion) {
                future.complete(completion);
            }

            @Override
            public void onError(String error) {
                future.completeExceptionally(new RuntimeException(error));
            }
        });

        return future;
    }

    /** Package-private: used by ToolCallingAgent and other SDK types. */
    long handle() {
        ensureOpen();
        return handle;
    }

    private void ensureOpen() {
        if (closed) {
            throw new IllegalStateException("ForgeClient is closed");
        }
    }

    @Override
    public void close() {
        if (!closed) {
            closed = true;
            nativeDestroy(handle);
        }
    }

    // Native methods
    private static native long nativeCreate();
    private static native long nativeCreateWithConfig(String apiKey, String baseUrl, String defaultModel, int timeoutSeconds);
    private static native void nativeDestroy(long handle);
    private static native String nativeGetModel(long handle);
    private static native String nativeGetBaseUrl(long handle);
    private static native ChatCompletion nativeComplete(long handle, List<Message> messages);
    private static native ChatCompletion nativeCompleteWithModel(long handle, String model, List<Message> messages);
    private static native void nativeCompleteAsync(long handle, List<Message> messages, CompletionCallback callback);
    private static native void nativeCompleteAsyncWithModel(long handle, String model, List<Message> messages, CompletionCallback callback);
}
