package com.liteforge;

import java.util.Objects;

/**
 * Represents a message in a chat conversation.
 */
public final class Message {
    private final String role;
    private final String content;

    /**
     * Creates a new message.
     *
     * @param role    The role of the message sender (e.g., "user", "assistant", "system")
     * @param content The content of the message
     */
    public Message(String role, String content) {
        this.role = Objects.requireNonNull(role, "role cannot be null");
        this.content = content;
    }

    /**
     * Creates a user message.
     *
     * @param content The message content
     * @return A new user message
     */
    public static Message user(String content) {
        return new Message("user", content);
    }

    /**
     * Creates an assistant message.
     *
     * @param content The message content
     * @return A new assistant message
     */
    public static Message assistant(String content) {
        return new Message("assistant", content);
    }

    /**
     * Creates a system message.
     *
     * @param content The message content
     * @return A new system message
     */
    public static Message system(String content) {
        return new Message("system", content);
    }

    public String getRole() {
        return role;
    }

    public String getContent() {
        return content;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        Message message = (Message) o;
        return Objects.equals(role, message.role) && Objects.equals(content, message.content);
    }

    @Override
    public int hashCode() {
        return Objects.hash(role, content);
    }

    @Override
    public String toString() {
        return "Message{role='" + role + "', content='" + content + "'}";
    }
}
