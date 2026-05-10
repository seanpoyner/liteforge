package com.liteforge;

/**
 * A document stored in the knowledge base.
 *
 * <p>{@code metadataJson} and {@code content} are passed verbatim across the
 * JNI boundary — no transformation. Keep metadata as a JSON object string if
 * you use it (e.g. {@code "{\"author\":\"alice\"}"}).
 */
public final class Document {
    private final String id;
    private final String content;
    private String namespace;
    private String source;
    private String metadataJson;

    public Document(String id, String content) {
        if (id == null || id.isEmpty()) throw new IllegalArgumentException("id must be non-empty");
        if (content == null) throw new NullPointerException("content");
        this.id = id;
        this.content = content;
    }

    public Document withNamespace(String namespace) {
        this.namespace = namespace;
        return this;
    }

    public Document withSource(String source) {
        this.source = source;
        return this;
    }

    /** Set metadata as a JSON object string. */
    public Document withMetadataJson(String metadataJson) {
        this.metadataJson = metadataJson;
        return this;
    }

    public String getId() { return id; }
    public String getContent() { return content; }
    public String getNamespace() { return namespace; }
    public String getSource() { return source; }
    public String getMetadataJson() { return metadataJson; }

    String toJson() {
        StringBuilder sb = new StringBuilder();
        sb.append('{');
        sb.append("\"id\":\"").append(esc(id)).append("\",");
        sb.append("\"content\":\"").append(esc(content)).append('"');
        if (namespace != null) {
            sb.append(",\"namespace\":\"").append(esc(namespace)).append('"');
        }
        if (source != null) {
            sb.append(",\"source\":\"").append(esc(source)).append('"');
        }
        sb.append(",\"metadata\":").append(metadataJson != null ? metadataJson : "{}");
        sb.append('}');
        return sb.toString();
    }

    /**
     * Parse a JSON object produced by the Rust side back into a {@link Document}.
     *
     * <p>Handles the common fields (id, content, namespace, source); metadata
     * is preserved as raw JSON so Java callers can parse it with their library
     * of choice.
     */
    static Document fromJson(String json) {
        // Minimal parser — we trust the Rust side's serde output shape.
        String id = extractString(json, "\"id\":");
        String content = extractString(json, "\"content\":");
        if (id == null || content == null) {
            throw new IllegalArgumentException("invalid Document JSON: " + json);
        }
        Document d = new Document(id, content);
        String ns = extractString(json, "\"namespace\":");
        if (ns != null) d.namespace = ns;
        String src = extractString(json, "\"source\":");
        if (src != null) d.source = src;
        String meta = extractObject(json, "\"metadata\":");
        if (meta != null) d.metadataJson = meta;
        return d;
    }

    private static String extractString(String json, String key) {
        int i = json.indexOf(key);
        if (i < 0) return null;
        int start = i + key.length();
        while (start < json.length() && Character.isWhitespace(json.charAt(start))) start++;
        if (start >= json.length() || json.charAt(start) != '"') return null;
        start++;
        StringBuilder sb = new StringBuilder();
        boolean escape = false;
        while (start < json.length()) {
            char c = json.charAt(start++);
            if (escape) {
                switch (c) {
                    case 'n': sb.append('\n'); break;
                    case 'r': sb.append('\r'); break;
                    case 't': sb.append('\t'); break;
                    case '"': sb.append('"'); break;
                    case '\\': sb.append('\\'); break;
                    default: sb.append(c);
                }
                escape = false;
            } else if (c == '\\') {
                escape = true;
            } else if (c == '"') {
                return sb.toString();
            } else {
                sb.append(c);
            }
        }
        return null;
    }

    private static String extractObject(String json, String key) {
        int i = json.indexOf(key);
        if (i < 0) return null;
        int start = i + key.length();
        while (start < json.length() && Character.isWhitespace(json.charAt(start))) start++;
        if (start >= json.length() || json.charAt(start) != '{') return null;
        int depth = 0;
        int begin = start;
        while (start < json.length()) {
            char c = json.charAt(start);
            if (c == '{') depth++;
            else if (c == '}') { depth--; if (depth == 0) return json.substring(begin, start + 1); }
            start++;
        }
        return null;
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
