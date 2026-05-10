package com.liteforge;

/**
 * Builder for creating ForgeClient instances with custom configuration.
 */
public final class ForgeClientBuilder {
    private String apiKey;
    private String baseUrl;
    private String defaultModel;
    private int timeoutSeconds = 30;

    ForgeClientBuilder() {
    }

    /**
     * Sets the API key for authentication.
     *
     * @param apiKey The API key
     * @return This builder
     */
    public ForgeClientBuilder apiKey(String apiKey) {
        this.apiKey = apiKey;
        return this;
    }

    /**
     * Sets the base URL for the API.
     *
     * @param baseUrl The base URL
     * @return This builder
     */
    public ForgeClientBuilder baseUrl(String baseUrl) {
        this.baseUrl = baseUrl;
        return this;
    }

    /**
     * Sets the default model to use for completions.
     *
     * @param defaultModel The default model name
     * @return This builder
     */
    public ForgeClientBuilder defaultModel(String defaultModel) {
        this.defaultModel = defaultModel;
        return this;
    }

    /**
     * Sets the request timeout in seconds.
     *
     * @param timeoutSeconds The timeout in seconds
     * @return This builder
     */
    public ForgeClientBuilder timeoutSeconds(int timeoutSeconds) {
        this.timeoutSeconds = timeoutSeconds;
        return this;
    }

    /**
     * Builds the ForgeClient with the configured settings.
     *
     * @return A new ForgeClient instance
     */
    public ForgeClient build() {
        return new ForgeClient(apiKey, baseUrl, defaultModel, timeoutSeconds);
    }

    String getApiKey() {
        return apiKey;
    }

    String getBaseUrl() {
        return baseUrl;
    }

    String getDefaultModel() {
        return defaultModel;
    }

    int getTimeoutSeconds() {
        return timeoutSeconds;
    }
}
