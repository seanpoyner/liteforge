package com.liteforge;

/**
 * A search hit returned by {@link LocalKnowledgeBackend#search(String)}.
 */
public final class SearchResult {
    private final Document document;
    private final float score;

    SearchResult(Document document, float score) {
        this.document = document;
        this.score = score;
    }

    public Document getDocument() {
        return document;
    }

    public float getScore() {
        return score;
    }

    @Override
    public String toString() {
        return "SearchResult{id=" + document.getId() + ", score=" + score + "}";
    }
}
