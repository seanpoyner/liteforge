package com.liteforge;

import java.util.Objects;

/**
 * A single choice in a chat completion response.
 */
public final class Choice {
    private final int index;
    private final Message message;
    private final String finishReason;

    public Choice(int index, Message message, String finishReason) {
        this.index = index;
        this.message = message;
        this.finishReason = finishReason;
    }

    public int getIndex() {
        return index;
    }

    public Message getMessage() {
        return message;
    }

    public String getFinishReason() {
        return finishReason;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        Choice choice = (Choice) o;
        return index == choice.index &&
               Objects.equals(message, choice.message) &&
               Objects.equals(finishReason, choice.finishReason);
    }

    @Override
    public int hashCode() {
        return Objects.hash(index, message, finishReason);
    }

    @Override
    public String toString() {
        return "Choice{index=" + index +
               ", message=" + message +
               ", finishReason='" + finishReason + "'}";
    }
}
