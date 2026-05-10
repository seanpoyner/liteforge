# LiteForge Java

Java bindings for the LiteForge SDK.

## Requirements

- Java 8, 11, 17, or 21
- Gradle 8.x (for building)

## Installation

### Gradle

```groovy
dependencies {
    implementation 'com.liteforge:liteforge-java:0.1.0'
}
```

### Maven

```xml
<dependency>
    <groupId>com.liteforge</groupId>
    <artifactId>liteforge-java</artifactId>
    <version>0.1.0</version>
</dependency>
```

## Quick Start

```java
import com.liteforge.*;
import java.util.List;

public class Example {
    public static void main(String[] args) {
        // Create client (reads LITEFORGE_API_KEY from environment)
        try (ForgeClient client = new ForgeClient()) {
            // Make a completion request
            ChatCompletion response = client.complete(List.of(
                Message.user("What is the capital of France?")
            ));

            System.out.println(response.getContent());
        }
    }
}
```

## Configuration

### Environment Variables

- `LITEFORGE_API_KEY` or `OPENAI_API_KEY`: API key for authentication
- `LITEFORGE_BASE_URL`: Custom API endpoint
- `LITEFORGE_DEFAULT_MODEL`: Default model to use

### Builder Pattern

```java
ForgeClient client = ForgeClient.builder()
    .apiKey("your-api-key")
    .baseUrl("https://api.example.com")
    .defaultModel("gpt-4")
    .timeoutSeconds(30)
    .build();
```

## Async Usage

The SDK supports async operations using `CompletableFuture`:

```java
ForgeClient client = new ForgeClient();

CompletableFuture<ChatCompletion> future = client.completeAsync(
    List.of(Message.user("Hello!"))
);

future.thenAccept(response -> {
    System.out.println(response.getContent());
});

// Or block for the result
ChatCompletion response = future.get();
```

## API Reference

### ForgeClient

| Method | Description |
|--------|-------------|
| `complete(List<Message> messages)` | Synchronous completion with default model |
| `complete(String model, List<Message> messages)` | Synchronous completion with specified model |
| `completeAsync(List<Message> messages)` | Async completion returning CompletableFuture |
| `completeAsync(String model, List<Message> messages)` | Async completion with specified model |
| `getModel()` | Get the default model name |
| `getBaseUrl()` | Get the API base URL |
| `close()` | Release native resources |

### Message

| Method | Description |
|--------|-------------|
| `Message.user(String content)` | Create a user message |
| `Message.assistant(String content)` | Create an assistant message |
| `Message.system(String content)` | Create a system message |
| `getRole()` | Get the message role |
| `getContent()` | Get the message content |

### ChatCompletion

| Method | Description |
|--------|-------------|
| `getId()` | Get the completion ID |
| `getModel()` | Get the model used |
| `getChoices()` | Get the list of choices |
| `getUsage()` | Get token usage statistics |
| `getContent()` | Convenience method to get first choice content |

## Building from Source

```bash
# Build the native library
cargo build -p liteforge-java --release

# Build the Java package
cd crates/liteforge-java
./gradlew build

# Run tests
./gradlew test
```

## Native Library Loading

The SDK automatically loads the native library from:
1. System library path (`java.library.path`)
2. JAR resources (`/native/{os}-{arch}/libliteforge_java.{so|dylib|dll}`)

## License

MIT License
