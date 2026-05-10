package com.liteforge;

import java.util.List;
import java.util.Objects;

/**
 * A chat completion response from the API.
 */
public final class ChatCompletion {
    private final String id;
    private final String model;
    private final long created;
    private final List<Choice> choices;
    private final Usage usage;

    public ChatCompletion(String id, String model, long created, List<Choice> choices, Usage usage) {
        this.id = id;
        this.model = model;
        this.created = created;
        this.choices = choices;
        this.usage = usage;
    }

    public String getId() {
        return id;
    }

    public String getModel() {
        return model;
    }

    public long getCreated() {
        return created;
    }

    public List<Choice> getChoices() {
        return choices;
    }

    public Usage getUsage() {
        return usage;
    }

    /**
     * Convenience method to get the content of the first choice.
     *
     * @return The content of the first choice, or null if no choices exist
     */
    public String getContent() {
        if (choices == null || choices.isEmpty()) {
            return null;
        }
        Message message = choices.get(0).getMessage();
        return message != null ? message.getContent() : null;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        ChatCompletion that = (ChatCompletion) o;
        return created == that.created &&
               Objects.equals(id, that.id) &&
               Objects.equals(model, that.model) &&
               Objects.equals(choices, that.choices) &&
               Objects.equals(usage, that.usage);
    }

    @Override
    public int hashCode() {
        return Objects.hash(id, model, created, choices, usage);
    }

    @Override
    public String toString() {
        return "ChatCompletion{id='" + id + "', model='" + model +
               "', created=" + created + ", choices=" + choices +
               ", usage=" + usage + "}";
    }
}
