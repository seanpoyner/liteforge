package com.liteforge;

/**
 * Callback for asynchronous agent execution.
 * Invoked from a native thread — do not block.
 */
public interface AgentCallback {
    void onSuccess(String response);

    void onError(String error);
}
