package com.liteforge;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Configuration for an MCP server connection. Serializes to the same JSON
 * shape as {@code liteforge::mcp::McpServerConfig}.
 *
 * <p>Use the transport-specific factory methods:
 * <ul>
 *   <li>{@link #stdio(String, String)} — subprocess over stdin/stdout</li>
 *   <li>{@link #http(String, String)} — REST/HTTP</li>
 *   <li>{@link #sse(String, String)} — Server-Sent Events</li>
 * </ul>
 *
 * <pre>{@code
 * McpServerConfig fs = McpServerConfig.stdio("filesystem", "npx")
 *     .withArg("-y")
 *     .withArg("@modelcontextprotocol/server-filesystem")
 *     .withArg("/tmp");
 * }</pre>
 */
public final class McpServerConfig {
    public enum Transport { STDIO, HTTP, SSE }

    private final String name;
    private final Transport transport;
    private String command;
    private final List<String> args = new ArrayList<>();
    private final Map<String, String> env = new LinkedHashMap<>();
    private String cwd;
    private String url;
    private String bearerToken;
    private String apiKeyHeader;
    private String apiKeyValue;

    private McpServerConfig(String name, Transport transport) {
        this.name = name;
        this.transport = transport;
    }

    public static McpServerConfig stdio(String name, String command) {
        McpServerConfig c = new McpServerConfig(name, Transport.STDIO);
        c.command = command;
        return c;
    }

    public static McpServerConfig http(String name, String url) {
        McpServerConfig c = new McpServerConfig(name, Transport.HTTP);
        c.url = url;
        return c;
    }

    public static McpServerConfig sse(String name, String url) {
        McpServerConfig c = new McpServerConfig(name, Transport.SSE);
        c.url = url;
        return c;
    }

    public McpServerConfig withArg(String arg) {
        args.add(arg);
        return this;
    }

    public McpServerConfig withEnv(String key, String value) {
        env.put(key, value);
        return this;
    }

    public McpServerConfig withCwd(String cwd) {
        this.cwd = cwd;
        return this;
    }

    public McpServerConfig withBearerToken(String token) {
        this.bearerToken = token;
        return this;
    }

    public McpServerConfig withApiKey(String header, String value) {
        this.apiKeyHeader = header;
        this.apiKeyValue = value;
        return this;
    }

    public String getName() { return name; }
    public Transport getTransport() { return transport; }

    String toJson() {
        StringBuilder sb = new StringBuilder();
        sb.append('{');
        appendStr(sb, "name", name);
        sb.append(',');
        appendStr(sb, "transport", transport.name().toLowerCase());
        if (command != null) {
            sb.append(',');
            appendStr(sb, "command", command);
        }
        if (!args.isEmpty()) {
            sb.append(",\"args\":[");
            for (int i = 0; i < args.size(); i++) {
                if (i > 0) sb.append(',');
                sb.append('"').append(esc(args.get(i))).append('"');
            }
            sb.append(']');
        }
        if (!env.isEmpty()) {
            sb.append(",\"env\":{");
            int i = 0;
            for (Map.Entry<String, String> e : env.entrySet()) {
                if (i++ > 0) sb.append(',');
                sb.append('"').append(esc(e.getKey())).append("\":\"")
                        .append(esc(e.getValue())).append('"');
            }
            sb.append('}');
        }
        if (cwd != null) {
            sb.append(',');
            appendStr(sb, "cwd", cwd);
        }
        if (url != null) {
            sb.append(',');
            appendStr(sb, "url", url);
        }
        if (bearerToken != null) {
            sb.append(",\"auth\":{\"type\":\"bearer\",\"token\":\"")
                    .append(esc(bearerToken)).append("\"}");
        } else if (apiKeyHeader != null) {
            sb.append(",\"auth\":{\"type\":\"apikey\",\"header\":\"")
                    .append(esc(apiKeyHeader))
                    .append("\",\"key\":\"")
                    .append(esc(apiKeyValue))
                    .append("\"}");
        }
        sb.append('}');
        return sb.toString();
    }

    private static void appendStr(StringBuilder sb, String key, String value) {
        sb.append('"').append(key).append("\":\"").append(esc(value)).append('"');
    }

    private static String esc(String s) {
        StringBuilder out = new StringBuilder(s.length());
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            switch (c) {
                case '\\': out.append("\\\\"); break;
                case '"':  out.append("\\\""); break;
                case '\n': out.append("\\n"); break;
                case '\r': out.append("\\r"); break;
                case '\t': out.append("\\t"); break;
                default:
                    if (c < 0x20) out.append(String.format("\\u%04x", (int) c));
                    else out.append(c);
            }
        }
        return out.toString();
    }
}
