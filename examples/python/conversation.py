#!/usr/bin/env python3
"""
Conversation example - managed conversations with context windowing.

Run with: python examples/python/conversation.py

This example demonstrates:
- ManagedConversation for tracking messages
- ConversationConfig for configuring behavior
- CompactingConversation for automatic summarization
- Token estimation and context management

Note: Uses local conversation management with no external services,
no API keys required.
"""

from liteforge import (
    ManagedConversation,
    CompactingConversation,
    ConversationConfig,
    SummarizationStrategy,
)


def main():
    print("=== LiteForge Conversation Example (Python) ===\n")

    # 1. Basic ManagedConversation
    print("1. Basic ManagedConversation...")

    conv = ManagedConversation()
    conv.set_system("You are a helpful coding assistant.")
    conv.add_user_message("Hello! Can you help me with Rust?")
    conv.add_assistant_message("Of course! I'd be happy to help with Rust. What would you like to know?")
    conv.add_user_message("How do I create a vector?")
    conv.add_assistant_message("You can create a vector using `Vec::new()` or the `vec![]` macro.")

    print(f"   Messages: {len(conv)}")
    print(f"   Estimated tokens: ~{conv.estimated_tokens()}")
    print(f"   Has system message: {conv.system_message() is not None}")

    # 2. Access messages
    print("\n2. Accessing messages...")

    all_messages = conv.messages()
    print(f"   Total messages (including system): {len(all_messages)}")

    last_msg = conv.last_message()
    if last_msg:
        content = last_msg.content or ""
        print(f'   Last message: "{content[:40]}..."')

    # Get last N messages
    recent = conv.last_n_messages(2)
    print("   Last 2 messages:")
    for msg in recent:
        print(f"     - {msg.role}: {msg.content}")

    # 3. Export as text
    print("\n3. Exporting conversation...")

    text = conv.to_text()
    print("   Text representation:")
    for line in text.split('\n')[:6]:
        print(f"     {line}")
    print("     ...")

    # 4. ConversationConfig
    print("\n4. ConversationConfig options...")

    config = ConversationConfig()
    print(f"   Default max_tokens: {config.max_tokens}")
    print(f"   Default target_tokens: {config.target_tokens}")
    print(f"   Default preserve_recent: {config.preserve_recent}")
    print(f"   Default preserve_system: {config.preserve_system}")
    print(f"   Default strategy: {config.strategy}")

    # Modify config
    config.max_tokens = 8000
    config.target_tokens = 4000
    config.preserve_recent = 6
    print(f"\n   After modification:")
    print(f"     max_tokens: {config.max_tokens}")
    print(f"     target_tokens: {config.target_tokens}")
    print(f"     preserve_recent: {config.preserve_recent}")

    # 5. SummarizationStrategy options
    print("\n5. Available summarization strategies...")

    strategies = [
        (SummarizationStrategy.keep_recent(), "Keep only recent messages, discard old ones"),
        (SummarizationStrategy.summarize(), "Summarize older messages into system context"),
        (SummarizationStrategy.chunked_summary(), "Create multiple summary levels"),
        (SummarizationStrategy.preserve_system(), "Always preserve system messages"),
    ]

    for strategy, description in strategies:
        print(f"   {strategy}: {description}")

    # 6. CompactingConversation
    print("\n6. CompactingConversation with auto-summarization...")

    # Create config with small limits to trigger compaction
    compact_config = ConversationConfig()
    compact_config.max_tokens = 100
    compact_config.target_tokens = 50
    compact_config.preserve_recent = 2
    compact_config.strategy = SummarizationStrategy.summarize()

    compact_conv = CompactingConversation(compact_config)
    compact_conv.set_system("You are a helpful assistant.")

    # Add messages until compaction is needed
    print("   Adding messages...")
    for i in range(1, 11):
        compact_conv.add_user_message(f"This is user message number {i}. It has some content to use tokens.")
        compact_conv.add_assistant_message(f"Response to message {i}. Here is a helpful reply with information.")

    print(f"   Total messages added: {compact_conv.total_messages_added()}")
    print(f"   Current message count: {len(compact_conv)}")
    print(f"   Estimated tokens: ~{compact_conv.estimated_tokens()}")
    print(f"   Needs compaction: {compact_conv.needs_compaction()}")

    # 7. Perform compaction
    print("\n7. Performing compaction...")

    if compact_conv.needs_compaction():
        result = compact_conv.compact()
        if result:
            print("   Compaction result:")
            print(f"     Messages removed: {result.messages_removed}")
            print(f"     Tokens before: {result.tokens_before}")
            print(f"     Tokens after: {result.tokens_after}")
            if result.summary:
                print(f'     Summary preview: "{result.summary[:60]}..."')

    print("   After compaction:")
    print(f"     Message count: {len(compact_conv)}")
    print(f"     Compaction count: {compact_conv.compaction_count()}")
    print(f"     Summaries stored: {len(compact_conv.summaries())}")

    # 8. Get messages for API
    print("\n8. Getting messages for API call...")

    api_messages = compact_conv.messages()
    print(f"   Messages ready for API: {len(api_messages)}")
    for msg in api_messages[:5]:
        content = msg.content or ""
        preview = content[:50]
        print(f'     - [{msg.role}] "{preview}..."')

    # 9. Practical workflow
    print("\n9. Practical conversation workflow...")

    def simulate_conversation():
        # Configure for a typical use case
        config = ConversationConfig()
        config.max_tokens = 4000
        config.target_tokens = 2000
        config.preserve_recent = 6
        config.strategy = SummarizationStrategy.summarize()

        conv = CompactingConversation(config)
        conv.set_system("You are a helpful assistant that remembers our conversation.")

        # Simulate a multi-turn conversation
        exchanges = [
            ("What's Rust?", "Rust is a systems programming language focused on safety and performance."),
            ("What are its main features?", "Key features include memory safety without GC, zero-cost abstractions, and fearless concurrency."),
            ("How do I install it?", "Use rustup: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"),
            ("What's cargo?", "Cargo is Rust's build system and package manager."),
        ]

        for user_msg, assistant_msg in exchanges:
            conv.add_user_message(user_msg)
            conv.add_assistant_message(assistant_msg)

            # Check if compaction needed after each exchange
            if conv.needs_compaction():
                conv.compact()

        print("   Conversation summary:")
        print(f"     Exchanges: {len(exchanges)}")
        print(f"     Messages retained: {len(conv)}")
        print(f"     Compactions: {conv.compaction_count()}")

    simulate_conversation()

    # 10. Clear operations
    print("\n10. Clearing conversations...")

    # Create a new conversation for clearing demo
    conv = ManagedConversation()
    conv.set_system("System prompt")
    conv.add_user_message("Hello")
    conv.add_assistant_message("Hi!")

    # Clear but keep system message
    conv.clear(keep_system=True)
    print("   After clear(keep_system=True):")
    print(f"     Messages: {len(conv)}")
    print(f"     System message preserved: {conv.system_message() is not None}")

    # Full clear
    conv.clear(keep_system=False)
    print("   After clear(keep_system=False):")
    print(f"     Messages: {len(conv)}")
    print(f"     System message: {conv.system_message() is not None}")

    # 11. ManagedConversation repr
    print("\n11. Conversation representations...")

    conv = ManagedConversation()
    conv.add_user_message("Hello")
    conv.add_assistant_message("Hi there!")
    print(f"   ManagedConversation: {conv}")

    compact = CompactingConversation()
    compact.add_user_message("Test message")
    print(f"   CompactingConversation: {compact}")

    config = ConversationConfig()
    print(f"   ConversationConfig: {config}")

    # 12. Building conversations programmatically
    print("\n12. Building conversations programmatically...")

    def build_conversation_from_history(history: list[tuple[str, str]]) -> ManagedConversation:
        """Build a conversation from a list of (role, content) tuples."""
        conv = ManagedConversation()
        for role, content in history:
            if role == "system":
                conv.set_system(content)
            elif role == "user":
                conv.add_user_message(content)
            elif role == "assistant":
                conv.add_assistant_message(content)
        return conv

    history = [
        ("system", "You are a code reviewer."),
        ("user", "Please review my Python code."),
        ("assistant", "I'd be happy to help. Please share the code."),
        ("user", "def add(a, b): return a + b"),
        ("assistant", "The function looks good! Consider adding type hints."),
    ]

    conv = build_conversation_from_history(history)
    print(f"   Built conversation with {len(conv)} messages")
    print(f"   System message: {conv.system_message().content if conv.system_message() else 'None'}")

    print("\n=== Example Complete ===")


if __name__ == "__main__":
    main()
