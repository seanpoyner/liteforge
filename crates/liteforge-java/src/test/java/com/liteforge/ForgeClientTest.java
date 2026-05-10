package com.liteforge;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeAll;
import static org.junit.jupiter.api.Assertions.*;

import java.util.Arrays;
import java.util.List;

class ForgeClientTest {

    @Test
    void testMessageCreation() {
        Message userMsg = Message.user("Hello");
        assertEquals("user", userMsg.getRole());
        assertEquals("Hello", userMsg.getContent());

        Message assistantMsg = Message.assistant("Hi there");
        assertEquals("assistant", assistantMsg.getRole());

        Message systemMsg = Message.system("You are helpful");
        assertEquals("system", systemMsg.getRole());
    }

    @Test
    void testUsageCreation() {
        Usage usage = new Usage(10, 20, 30);
        assertEquals(10, usage.getPromptTokens());
        assertEquals(20, usage.getCompletionTokens());
        assertEquals(30, usage.getTotalTokens());
    }

    @Test
    void testChoiceCreation() {
        Message msg = Message.assistant("Response");
        Choice choice = new Choice(0, msg, "stop");
        assertEquals(0, choice.getIndex());
        assertEquals("stop", choice.getFinishReason());
        assertEquals(msg, choice.getMessage());
    }

    @Test
    void testChatCompletionCreation() {
        Message msg = Message.assistant("Response");
        Choice choice = new Choice(0, msg, "stop");
        Usage usage = new Usage(10, 20, 30);

        ChatCompletion completion = new ChatCompletion(
            "id-123",
            "gpt-4",
            1234567890L,
            Arrays.asList(choice),
            usage
        );

        assertEquals("id-123", completion.getId());
        assertEquals("gpt-4", completion.getModel());
        assertEquals("Response", completion.getContent());
    }

    @Test
    void testBuilderPattern() {
        ForgeClientBuilder builder = ForgeClient.builder()
            .apiKey("test-key")
            .baseUrl("https://api.example.com")
            .defaultModel("gpt-4")
            .timeoutSeconds(60);

        assertEquals("test-key", builder.getApiKey());
        assertEquals("https://api.example.com", builder.getBaseUrl());
        assertEquals("gpt-4", builder.getDefaultModel());
        assertEquals(60, builder.getTimeoutSeconds());
    }
}
