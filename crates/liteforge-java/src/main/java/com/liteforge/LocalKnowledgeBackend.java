package com.liteforge;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * In-memory knowledge backend for RAG. Uses simple term-frequency scoring —
 * suitable for CodeFest demos and development, not for production search at
 * scale.
 *
 * <pre>{@code
 * try (ForgeClient client = new ForgeClient();
 *      LocalKnowledgeBackend kb = new LocalKnowledgeBackend(client)) {
 *     kb.upload(List.of(
 *         new Document("1", "Rust is a systems programming language"),
 *         new Document("2", "Python is great for data science")
 *     ));
 *     for (SearchResult hit : kb.search("rust programming", 5)) {
 *         System.out.println(hit);
 *     }
 * }
 * }</pre>
 */
public final class LocalKnowledgeBackend implements AutoCloseable {
    static {
        NativeLibrary.ensureLoaded();
    }

    private final long handle;
    private volatile boolean closed = false;

    public LocalKnowledgeBackend(ForgeClient client) {
        if (client == null) throw new NullPointerException("client");
        this.handle = nativeCreate(client.handle());
        if (this.handle == 0) {
            throw new RuntimeException("Failed to create LocalKnowledgeBackend");
        }
    }

    /** Upload one document and return the count stored. */
    public int upload(Document document) {
        return upload(Collections.singletonList(document));
    }

    /** Upload a batch of documents. Returns the number accepted. */
    public int upload(List<Document> documents) {
        ensureOpen();
        if (documents == null || documents.isEmpty()) return 0;
        StringBuilder sb = new StringBuilder();
        sb.append('[');
        for (int i = 0; i < documents.size(); i++) {
            if (i > 0) sb.append(',');
            sb.append(documents.get(i).toJson());
        }
        sb.append(']');
        return nativeUpload(handle, sb.toString());
    }

    /** Search. Returns up to the default number of results. */
    public List<SearchResult> search(String query) {
        return search(query, 0, null);
    }

    public List<SearchResult> search(String query, int limit) {
        return search(query, limit, null);
    }

    public List<SearchResult> search(String query, int limit, String namespace) {
        ensureOpen();
        if (query == null) throw new NullPointerException("query");
        String json = nativeSearch(handle, query, limit, namespace);
        return parseSearchResults(json);
    }

    /** Get a document by ID, or {@code null} if not present. */
    public Document get(String id) {
        ensureOpen();
        if (id == null) throw new NullPointerException("id");
        String json = nativeGet(handle, id);
        if (json == null || json.isEmpty()) return null;
        return Document.fromJson(json);
    }

    /** Delete by ID. Returns true if the document existed. */
    public boolean delete(String id) {
        ensureOpen();
        if (id == null) throw new NullPointerException("id");
        return nativeDelete(handle, id);
    }

    /**
     * Number of documents currently stored. Parses the underlying stats JSON
     * field {@code "document_count"}.
     */
    public int size() {
        ensureOpen();
        String json = nativeStatsJson(handle);
        int i = json.indexOf("\"document_count\":");
        if (i < 0) return 0;
        int start = i + "\"document_count\":".length();
        int end = start;
        while (end < json.length() && Character.isDigit(json.charAt(end))) end++;
        if (end == start) return 0;
        return Integer.parseInt(json.substring(start, end));
    }

    public List<Document> list(int limit, int offset, String namespace) {
        ensureOpen();
        String json = nativeListJson(handle, limit, offset, namespace);
        return parseDocumentArray(json);
    }

    private void ensureOpen() {
        if (closed) {
            throw new IllegalStateException("LocalKnowledgeBackend is closed");
        }
    }

    @Override
    public void close() {
        if (!closed) {
            closed = true;
            nativeDestroy(handle);
        }
    }

    private static List<SearchResult> parseSearchResults(String json) {
        List<SearchResult> out = new ArrayList<>();
        if (json == null || json.isEmpty() || json.equals("[]")) return out;
        // Walk the array by matching balanced {}.
        int i = 0;
        while (i < json.length()) {
            while (i < json.length() && json.charAt(i) != '{') i++;
            if (i >= json.length()) break;
            int start = i;
            int depth = 0;
            while (i < json.length()) {
                char c = json.charAt(i);
                if (c == '{') depth++;
                else if (c == '}') { depth--; if (depth == 0) { i++; break; } }
                i++;
            }
            String obj = json.substring(start, i);
            int scoreIdx = obj.indexOf("\"score\":");
            float score = 0f;
            if (scoreIdx >= 0) {
                int s = scoreIdx + "\"score\":".length();
                int e = s;
                while (e < obj.length() && "0123456789.-eE".indexOf(obj.charAt(e)) >= 0) e++;
                if (e > s) score = Float.parseFloat(obj.substring(s, e));
            }
            int docIdx = obj.indexOf("\"document\":");
            if (docIdx < 0) continue;
            int docStart = obj.indexOf('{', docIdx);
            int d = 0;
            int docEnd = docStart;
            while (docEnd < obj.length()) {
                char c = obj.charAt(docEnd);
                if (c == '{') d++;
                else if (c == '}') { d--; if (d == 0) { docEnd++; break; } }
                docEnd++;
            }
            Document doc = Document.fromJson(obj.substring(docStart, docEnd));
            out.add(new SearchResult(doc, score));
        }
        return out;
    }

    private static List<Document> parseDocumentArray(String json) {
        List<Document> out = new ArrayList<>();
        if (json == null || json.isEmpty() || json.equals("[]")) return out;
        int i = 0;
        while (i < json.length()) {
            while (i < json.length() && json.charAt(i) != '{') i++;
            if (i >= json.length()) break;
            int start = i;
            int depth = 0;
            while (i < json.length()) {
                char c = json.charAt(i);
                if (c == '{') depth++;
                else if (c == '}') { depth--; if (depth == 0) { i++; break; } }
                i++;
            }
            out.add(Document.fromJson(json.substring(start, i)));
        }
        return out;
    }

    private static native long nativeCreate(long clientHandle);
    private static native void nativeDestroy(long handle);
    private static native int nativeUpload(long handle, String docsJson);
    private static native String nativeSearch(long handle, String query, int limit, String namespace);
    private static native String nativeGet(long handle, String id);
    private static native boolean nativeDelete(long handle, String id);
    private static native String nativeStatsJson(long handle);
    private static native String nativeListJson(long handle, int limit, int offset, String namespace);
}
