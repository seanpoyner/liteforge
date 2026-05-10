package com.liteforge;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * Configuration for a {@link ToolCallingAgent}. Serializes to the same JSON
 * shape as {@code liteforge::agents::AgentConfig} on the Rust side.
 *
 * <p>Build with the fluent {@code with*} setters, then pass to
 * {@link ToolCallingAgent#withConfig(AgentConfig)}.
 */
public final class AgentConfig {
    private String name;
    private String systemPrompt;
    private String model;
    private int maxSteps = 10;
    private Float temperature;
    private Integer maxTokens;
    private boolean streaming = false;
    private final List<String> tools = new ArrayList<>();

    public AgentConfig(String name) {
        if (name == null || name.isEmpty()) {
            throw new IllegalArgumentException("agent name must be non-empty");
        }
        this.name = name;
    }

    public AgentConfig withSystemPrompt(String prompt) {
        this.systemPrompt = prompt;
        return this;
    }

    public AgentConfig withModel(String model) {
        this.model = model;
        return this;
    }

    public AgentConfig withMaxSteps(int maxSteps) {
        if (maxSteps < 1) {
            throw new IllegalArgumentException("maxSteps must be >= 1");
        }
        this.maxSteps = maxSteps;
        return this;
    }

    public AgentConfig withTemperature(float temperature) {
        this.temperature = temperature;
        return this;
    }

    public AgentConfig withMaxTokens(int maxTokens) {
        this.maxTokens = maxTokens;
        return this;
    }

    public AgentConfig withStreaming(boolean streaming) {
        this.streaming = streaming;
        return this;
    }

    public AgentConfig withTool(String toolName) {
        if (toolName == null || toolName.isEmpty()) {
            throw new IllegalArgumentException("tool name must be non-empty");
        }
        this.tools.add(toolName);
        return this;
    }

    public AgentConfig withTools(List<String> toolNames) {
        if (toolNames != null) {
            this.tools.addAll(toolNames);
        }
        return this;
    }

    public String getName() { return name; }
    public String getSystemPrompt() { return systemPrompt; }
    public String getModel() { return model; }
    public int getMaxSteps() { return maxSteps; }
    public Float getTemperature() { return temperature; }
    public Integer getMaxTokens() { return maxTokens; }
    public boolean isStreaming() { return streaming; }
    public List<String> getTools() { return Collections.unmodifiableList(tools); }

    /**
     * Serialize to JSON for passing to the native agent constructor.
     *
     * <p>Implemented by hand to avoid a Jackson/Gson dependency on the SDK
     * core. If you add a JSON library later, feel free to replace this.
     */
    String toJson() {
        StringBuilder sb = new StringBuilder();
        sb.append('{');
        appendString(sb, "name", name);
        if (systemPrompt != null) {
            sb.append(',');
            appendString(sb, "system_prompt", systemPrompt);
        }
        if (model != null) {
            sb.append(',');
            appendString(sb, "model", model);
        }
        sb.append(",\"max_steps\":").append(maxSteps);
        if (temperature != null) {
            sb.append(",\"temperature\":").append(temperature);
        }
        if (maxTokens != null) {
            sb.append(",\"max_tokens\":").append(maxTokens);
        }
        sb.append(",\"streaming\":").append(streaming);
        sb.append(",\"tools\":[");
        for (int i = 0; i < tools.size(); i++) {
            if (i > 0) sb.append(',');
            sb.append('"').append(jsonEscape(tools.get(i))).append('"');
        }
        sb.append(']');
        sb.append(",\"metadata\":{}");
        sb.append('}');
        return sb.toString();
    }

    private static void appendString(StringBuilder sb, String key, String value) {
        sb.append('"').append(key).append("\":\"").append(jsonEscape(value)).append('"');
    }

    private static String jsonEscape(String s) {
        StringBuilder out = new StringBuilder(s.length());
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            switch (c) {
                case '\\': out.append("\\\\"); break;
                case '"':  out.append("\\\""); break;
                case '\n': out.append("\\n"); break;
                case '\r': out.append("\\r"); break;
                case '\t': out.append("\\t"); break;
                case '\b': out.append("\\b"); break;
                case '\f': out.append("\\f"); break;
                default:
                    if (c < 0x20) {
                        out.append(String.format("\\u%04x", (int) c));
                    } else {
                        out.append(c);
                    }
            }
        }
        return out.toString();
    }
}
