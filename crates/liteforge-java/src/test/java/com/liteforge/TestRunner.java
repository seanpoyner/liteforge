package com.liteforge;

import java.util.Arrays;
import java.util.List;

public class TestRunner {
    public static void main(String[] args) {
        System.out.println("=== LiteForge Java Test Runner ===\n");

        int passed = 0;
        int failed = 0;

        // Test 1: Message creation
        try {
            System.out.print("Test 1: Message creation... ");
            Message msg = Message.user("Hello");
            assert "user".equals(msg.getRole()) : "Role should be 'user'";
            assert "Hello".equals(msg.getContent()) : "Content should be 'Hello'";
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            failed++;
        }

        // Test 2: Usage creation
        try {
            System.out.print("Test 2: Usage creation... ");
            Usage usage = new Usage(10, 20, 30);
            assert usage.getPromptTokens() == 10 : "Prompt tokens should be 10";
            assert usage.getCompletionTokens() == 20 : "Completion tokens should be 20";
            assert usage.getTotalTokens() == 30 : "Total tokens should be 30";
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            failed++;
        }

        // Test 3: ChatCompletion
        try {
            System.out.print("Test 3: ChatCompletion... ");
            Message msg = Message.assistant("Response");
            Choice choice = new Choice(0, msg, "stop");
            Usage usage = new Usage(10, 20, 30);
            ChatCompletion completion = new ChatCompletion("id-123", "gpt-4", 123456789L, Arrays.asList(choice), usage);
            assert "Response".equals(completion.getContent()) : "Content should be 'Response'";
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            failed++;
        }

        // Test 4: Native library loading
        try {
            System.out.print("Test 4: Native library loading... ");
            ForgeClient client = new ForgeClient();
            System.out.println("PASSED (library loaded)");
            passed++;

            // Test 5: Get default model
            try {
                System.out.print("Test 5: Get default model... ");
                String model = client.getModel();
                System.out.println("PASSED (model: " + model + ")");
                passed++;
            } catch (Throwable t) {
                System.out.println("FAILED: " + t.getMessage());
                failed++;
            }

            // Test 6: Get base URL
            try {
                System.out.print("Test 6: Get base URL... ");
                String url = client.getBaseUrl();
                System.out.println("PASSED (url: " + url + ")");
                passed++;
            } catch (Throwable t) {
                System.out.println("FAILED: " + t.getMessage());
                failed++;
            }

            client.close();
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 7: Empty tool registry
        try {
            System.out.print("Test 7: Empty ToolRegistry... ");
            try (ToolRegistry registry = new ToolRegistry()) {
                assert registry.size() == 0 : "New registry should be empty";
                assert !registry.contains("nope") : "contains() should be false for missing tool";
            }
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 8: Register a Java tool and inspect metadata
        try {
            System.out.print("Test 8: Register Java tool... ");
            try (ToolRegistry registry = new ToolRegistry()) {
                registry.register(new AddTool());
                assert registry.size() == 1 : "Registry should contain 1 tool";
                assert registry.contains("add") : "Registry should contain 'add'";
                assert registry.names().contains("add") : "names() should include 'add'";
                String defs = registry.definitionsJson();
                assert defs.contains("\"name\":\"add\"") : "definitions JSON should include name";
            }
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 9: Execute tool via ToolExecutor
        try {
            System.out.print("Test 9: Execute tool... ");
            try (ToolRegistry registry = new ToolRegistry();
                 ToolExecutor exec = new ToolExecutor(registry)) {
                registry.register(new AddTool());
                ToolResult r = exec.execute("add", "{\"a\":2,\"b\":3}");
                assert r.isSuccess() : "Execution should succeed, err=" + r.getError();
                assert r.getResultJson() != null : "Result JSON should be present";
                assert r.getResultJson().contains("5") : "Result should contain sum=5: " + r.getResultJson();
                assert r.getExecutionTimeMs() != null : "Execution time should be recorded";
            }
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 10: Tool exception surfaced as failed ToolResult
        try {
            System.out.print("Test 10: Tool exception handling... ");
            try (ToolRegistry registry = new ToolRegistry();
                 ToolExecutor exec = new ToolExecutor(registry)) {
                registry.register(new Tool() {
                    public String name() { return "boom"; }
                    public String description() { return "always throws"; }
                    public String parametersSchemaJson() {
                        return "{\"type\":\"object\",\"properties\":{}}";
                    }
                    public String execute(String argsJson) throws Exception {
                        throw new RuntimeException("intentional failure");
                    }
                });
                ToolResult r = exec.execute("boom", "{}");
                assert !r.isSuccess() : "Execution should fail";
                assert r.getError() != null : "Error message should be populated";
                assert r.getError().contains("intentional failure")
                    : "Error should contain thrown message: " + r.getError();
            }
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 11: Unknown tool returns failure
        try {
            System.out.print("Test 11: Unknown tool... ");
            try (ToolRegistry registry = new ToolRegistry();
                 ToolExecutor exec = new ToolExecutor(registry)) {
                ToolResult r = exec.execute("nope", "{}");
                assert !r.isSuccess() : "Execution should fail";
                assert r.getError() != null && r.getError().contains("not found")
                    : "Error should say 'not found': " + r.getError();
            }
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 12a: AgentConfig JSON serialization shape
        try {
            System.out.print("Test 12a: AgentConfig.toJson... ");
            AgentConfig cfg = new AgentConfig("test-bot")
                .withSystemPrompt("hi\n\"there\"")
                .withModel("gpt-4")
                .withMaxSteps(7)
                .withTemperature(0.3f)
                .withMaxTokens(256)
                .withTool("add");
            String json = cfg.toJson();
            assert json.contains("\"name\":\"test-bot\"") : "name missing: " + json;
            assert json.contains("\"system_prompt\":\"hi\\n\\\"there\\\"\"")
                : "escaped system_prompt missing: " + json;
            assert json.contains("\"model\":\"gpt-4\"") : "model missing: " + json;
            assert json.contains("\"max_steps\":7") : "max_steps missing: " + json;
            assert json.contains("\"temperature\":0.3") : "temperature missing: " + json;
            assert json.contains("\"max_tokens\":256") : "max_tokens missing: " + json;
            assert json.contains("\"tools\":[\"add\"]") : "tools missing: " + json;
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 12b: Build ToolCallingAgent end-to-end (native construction)
        try {
            System.out.print("Test 12b: Build ToolCallingAgent... ");
            try (ForgeClient client = new ForgeClient();
                 ToolRegistry tools = new ToolRegistry()) {
                tools.register(new AddTool());
                AgentConfig cfg = new AgentConfig("math-bot")
                    .withSystemPrompt("You add numbers.")
                    .withMaxSteps(3)
                    .withTool("add");
                try (ToolCallingAgent agent = new ToolCallingAgent(client, tools).withConfig(cfg)) {
                    agent.reset();  // should be a no-op on a fresh agent
                    assert "math-bot".equals(agent.config().getName())
                        : "Agent name mismatch: " + agent.config().getName();
                }
            }
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 12c: McpServerConfig JSON
        try {
            System.out.print("Test 12c: McpServerConfig.toJson... ");
            McpServerConfig cfg = McpServerConfig.stdio("fs", "npx")
                .withArg("-y")
                .withArg("@modelcontextprotocol/server-filesystem")
                .withArg("/tmp")
                .withEnv("DEBUG", "1");
            String json = cfg.toJson();
            assert json.contains("\"name\":\"fs\"") : "name missing: " + json;
            assert json.contains("\"transport\":\"stdio\"") : "transport missing: " + json;
            assert json.contains("\"command\":\"npx\"") : "command missing: " + json;
            assert json.contains("\"args\":[\"-y\"") : "args missing: " + json;
            assert json.contains("\"env\":{\"DEBUG\":\"1\"}") : "env missing: " + json;
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 12d: McpServer construction + isConnected before connect
        try {
            System.out.print("Test 12d: McpServer construction... ");
            try (ForgeClient client = new ForgeClient();
                 McpServer server = new McpServer(client,
                     McpServerConfig.stdio("test", "echo").withArg("hi"))) {
                assert "test".equals(server.getName()) : "name mismatch";
                assert !server.isConnected() : "should not be connected before connect()";
            }
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 12e: Knowledge upload + search
        try {
            System.out.print("Test 12e: Knowledge upload+search... ");
            try (ForgeClient client = new ForgeClient();
                 LocalKnowledgeBackend kb = new LocalKnowledgeBackend(client)) {
                int uploaded = kb.upload(java.util.List.of(
                    new Document("1", "Rust is a systems programming language"),
                    new Document("2", "Python is great for data science"),
                    new Document("3", "Java runs on the JVM")
                ));
                assert uploaded == 3 : "Expected 3 uploads, got " + uploaded;
                assert kb.size() == 3 : "Size mismatch: " + kb.size();

                java.util.List<SearchResult> hits = kb.search("rust programming", 5);
                assert !hits.isEmpty() : "Expected at least one hit for 'rust programming'";
                assert "1".equals(hits.get(0).getDocument().getId())
                    : "Top hit should be doc 1, got " + hits.get(0).getDocument().getId();
                assert hits.get(0).getScore() > 0f : "Score should be positive";
            }
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 12f: Knowledge get + delete
        try {
            System.out.print("Test 12f: Knowledge get+delete... ");
            try (ForgeClient client = new ForgeClient();
                 LocalKnowledgeBackend kb = new LocalKnowledgeBackend(client)) {
                kb.upload(new Document("hello", "world"));
                Document got = kb.get("hello");
                assert got != null : "get('hello') returned null";
                assert "world".equals(got.getContent()) : "Content mismatch: " + got.getContent();

                assert kb.delete("hello") : "delete should return true";
                assert kb.get("hello") == null : "get after delete should return null";
                assert !kb.delete("hello") : "second delete should return false";
            }
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 12g: Knowledge with namespace
        try {
            System.out.print("Test 12g: Knowledge namespace filter... ");
            try (ForgeClient client = new ForgeClient();
                 LocalKnowledgeBackend kb = new LocalKnowledgeBackend(client)) {
                kb.upload(java.util.List.of(
                    new Document("a1", "apple banana").withNamespace("fruits"),
                    new Document("a2", "car bus plane").withNamespace("vehicles")
                ));
                java.util.List<SearchResult> fruitHits = kb.search("apple", 5, "fruits");
                assert !fruitHits.isEmpty() : "fruit namespace search should return hits";
                java.util.List<SearchResult> vehicleHits = kb.search("apple", 5, "vehicles");
                assert vehicleHits.isEmpty() : "vehicle namespace should not match 'apple'";
            }
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        // Test 12: Call ID is correlated back in the result
        try {
            System.out.print("Test 12: executeWithId correlates call_id... ");
            try (ToolRegistry registry = new ToolRegistry();
                 ToolExecutor exec = new ToolExecutor(registry)) {
                registry.register(new AddTool());
                ToolResult r = exec.executeWithId("call_42", "add", "{\"a\":1,\"b\":1}");
                assert r.isSuccess() : "Execution should succeed";
                assert "call_42".equals(r.getToolCallId())
                    : "tool_call_id should be 'call_42' but was: " + r.getToolCallId();
            }
            System.out.println("PASSED");
            passed++;
        } catch (Throwable t) {
            System.out.println("FAILED: " + t.getMessage());
            t.printStackTrace();
            failed++;
        }

        System.out.println("\n=== Results ===");
        System.out.println("Passed: " + passed);
        System.out.println("Failed: " + failed);

        System.exit(failed > 0 ? 1 : 0);
    }
}
