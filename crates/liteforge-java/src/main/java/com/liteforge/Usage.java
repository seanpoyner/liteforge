package com.liteforge;

import java.util.Objects;

/**
 * Token usage information for a completion request.
 */
public final class Usage {
    private final int promptTokens;
    private final int completionTokens;
    private final int totalTokens;

    public Usage(int promptTokens, int completionTokens, int totalTokens) {
        this.promptTokens = promptTokens;
        this.completionTokens = completionTokens;
        this.totalTokens = totalTokens;
    }

    public int getPromptTokens() {
        return promptTokens;
    }

    public int getCompletionTokens() {
        return completionTokens;
    }

    public int getTotalTokens() {
        return totalTokens;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        Usage usage = (Usage) o;
        return promptTokens == usage.promptTokens &&
               completionTokens == usage.completionTokens &&
               totalTokens == usage.totalTokens;
    }

    @Override
    public int hashCode() {
        return Objects.hash(promptTokens, completionTokens, totalTokens);
    }

    @Override
    public String toString() {
        return "Usage{promptTokens=" + promptTokens +
               ", completionTokens=" + completionTokens +
               ", totalTokens=" + totalTokens + "}";
    }
}
