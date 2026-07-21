package com.liteforge;

/**
 * Native model router built from a LiteLLM-compatible YAML config.
 *
 * <p>Selection is config-driven: choose a built-in selector (static, semantic,
 * MF, or remote classifier) in the YAML. Custom selectors are not implemented
 * across the JNI boundary.</p>
 *
 * <p>Example usage:</p>
 * <pre>{@code
 * try (Router router = Router.fromFile("router.yaml")) {
 *     String model = router.whichModel("prove this theorem");
 *     String decisionJson = router.route("prove this theorem");
 * }
 * }</pre>
 */
public final class Router implements AutoCloseable {
    private final long handle;
    private volatile boolean closed = false;

    static {
        NativeLibrary.ensureLoaded();
    }

    private Router(long handle) {
        this.handle = handle;
        if (handle == 0) {
            throw new RuntimeException("Failed to create Router");
        }
    }

    /** Build a router from a YAML string. */
    public static Router fromYaml(String yaml) {
        return new Router(nativeCreateFromYaml(yaml));
    }

    /** Build a router from a YAML file path. */
    public static Router fromFile(String path) throws java.io.IOException {
        String yaml = new String(
            java.nio.file.Files.readAllBytes(java.nio.file.Paths.get(path)),
            java.nio.charset.StandardCharsets.UTF_8);
        return new Router(nativeCreateFromYaml(yaml));
    }

    /** The concrete model id a prompt would route to. */
    public String whichModel(String prompt) {
        ensureOpen();
        return nativeWhichModel(handle, prompt);
    }

    /**
     * The full routing decision for a prompt, as a JSON string with fields:
     * group, model, base_url, strategy, score, fallback_chain.
     */
    public String route(String prompt) {
        ensureOpen();
        return nativeRouteJson(handle, prompt);
    }

    /** The model group names this router serves. */
    public String[] modelGroups() {
        ensureOpen();
        String joined = nativeModelGroups(handle);
        if (joined == null || joined.isEmpty()) {
            return new String[0];
        }
        return joined.split(",");
    }

    /** The load-balancing strategy name. */
    public String strategy() {
        ensureOpen();
        return nativeStrategy(handle);
    }

    private void ensureOpen() {
        if (closed) {
            throw new IllegalStateException("Router is closed");
        }
    }

    @Override
    public void close() {
        if (!closed) {
            closed = true;
            nativeDestroy(handle);
        }
    }

    private static native long nativeCreateFromYaml(String yaml);

    private static native void nativeDestroy(long handle);

    private static native String nativeWhichModel(long handle, String prompt);

    private static native String nativeRouteJson(long handle, String prompt);

    private static native String nativeStrategy(long handle);

    private static native String nativeModelGroups(long handle);
}
