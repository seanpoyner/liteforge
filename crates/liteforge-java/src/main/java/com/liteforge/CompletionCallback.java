package com.liteforge;

/**
 * Callback interface for async completion requests.
 */
public interface CompletionCallback {
    /**
     * Called when the completion succeeds.
     *
     * @param completion The chat completion result
     */
    void onSuccess(ChatCompletion completion);

    /**
     * Called when the completion fails.
     *
     * @param error The error message
     */
    void onError(String error);
}
