package com.liteforge;

import java.util.Arrays;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;

public class IntegrationTest {
    public static void main(String[] args) {
        System.out.println("=== LiteForge Java Integration Test ===\n");

        String apiKey = System.getenv("LITEFORGE_API_KEY");
        if (apiKey == null || apiKey.isEmpty()) {
            apiKey = System.getenv("OPENAI_API_KEY");
        }

        if (apiKey == null || apiKey.isEmpty()) {
            System.out.println("Skipping integration test: No API key found");
            System.out.println("Set LITEFORGE_API_KEY or OPENAI_API_KEY environment variable");
            return;
        }

        try (ForgeClient client = new ForgeClient()) {
            System.out.println("Client created successfully");
            System.out.println("Model: " + client.getModel());
            System.out.println("Base URL: " + client.getBaseUrl());

            // Test sync completion
            System.out.println("\n--- Testing sync completion ---");
            List<Message> messages = Arrays.asList(
                Message.system("You are a helpful assistant. Be very brief."),
                Message.user("What is 2+2? Reply with just the number.")
            );

            try {
                ChatCompletion response = client.complete(messages);
                System.out.println("Response ID: " + response.getId());
                System.out.println("Content: " + response.getContent());
                if (response.getUsage() != null) {
                    System.out.println("Tokens used: " + response.getUsage().getTotalTokens());
                }
            } catch (Exception e) {
                System.out.println("Sync completion failed: " + e.getMessage());
            }

            // Test async completion
            System.out.println("\n--- Testing async completion ---");
            try {
                CompletableFuture<ChatCompletion> future = client.completeAsync(
                    Arrays.asList(Message.user("Say hello in exactly 3 words."))
                );

                ChatCompletion asyncResponse = future.get(30, TimeUnit.SECONDS);
                System.out.println("Async Response: " + asyncResponse.getContent());
            } catch (Exception e) {
                System.out.println("Async completion failed: " + e.getMessage());
            }

            System.out.println("\n=== Integration Test Complete ===");
        } catch (Exception e) {
            System.out.println("Test failed: " + e.getMessage());
            e.printStackTrace();
        }
    }
}
