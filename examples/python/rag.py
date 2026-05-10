#!/usr/bin/env python3
"""
RAG example - vector search and retrieval-augmented generation.

Run with: python examples/python/rag.py

This example demonstrates:
- Creating and storing embedded documents
- Vector similarity functions (cosine, dot product, euclidean)
- VectorIndex for similarity search
- Building a simple retrieval pipeline

Note: Uses mock embeddings to demonstrate functionality without
requiring an embedding API. In production, you'd use real embeddings.
"""

import math
from liteforge import (
    cosine_similarity,
    dot_product,
    euclidean_distance,
    normalize,
    EmbeddedDocument,
    VectorIndex,
)


def mock_embedding(text: str) -> list[float]:
    """
    Generate a simple mock embedding for demonstration.
    In production, you'd use an embedding model (e.g., text-embedding-3-small).
    """
    # Simple hash-based mock embedding (for demonstration only)
    embedding = [0.0] * 8
    for i, c in enumerate(text):
        idx = i % 8
        embedding[idx] += ord(c) / 1000.0
    # Normalize to unit length
    return normalize(embedding)


def main():
    print("=== LiteForge RAG Example (Python) ===\n")

    # 1. Vector math utilities
    print("1. Vector similarity functions...")

    v1 = [1.0, 0.0, 0.0]
    v2 = [1.0, 0.0, 0.0]  # Same direction
    v3 = [0.0, 1.0, 0.0]  # Orthogonal
    v4 = [0.707, 0.707, 0.0]  # 45 degrees

    print("   Vectors:")
    print(f"     v1 = {v1}")
    print(f"     v2 = {v2}")
    print(f"     v3 = {v3}")
    print(f"     v4 = {v4}")

    print("\n   Cosine similarity:")
    print(f"     v1 · v2 (same) = {cosine_similarity(v1, v2):.3f}")
    print(f"     v1 · v3 (orthogonal) = {cosine_similarity(v1, v3):.3f}")
    print(f"     v1 · v4 (45°) = {cosine_similarity(v1, v4):.3f}")

    print("\n   Dot product:")
    print(f"     v1 · v2 = {dot_product(v1, v2):.3f}")
    print(f"     v1 · v4 = {dot_product(v1, v4):.3f}")

    print("\n   Euclidean distance:")
    print(f"     v1 ↔ v2 = {euclidean_distance(v1, v2):.3f}")
    print(f"     v1 ↔ v3 = {euclidean_distance(v1, v3):.3f}")

    print("\n   Normalization:")
    unnorm = [3.0, 4.0]
    normalized = normalize(unnorm)
    print(f"     {unnorm} → {[round(x, 3) for x in normalized]} (length 1.0)")

    # 2. Create embedded documents
    print("\n2. Creating embedded documents...")

    documents = [
        EmbeddedDocument(
            "doc1",
            "Rust is a systems programming language focused on safety and performance.",
            mock_embedding("Rust is a systems programming language focused on safety and performance."),
        ).with_metadata("source", "rust-lang.org").with_metadata("category", "programming"),

        EmbeddedDocument(
            "doc2",
            "Python is a versatile programming language great for beginners and data science.",
            mock_embedding("Python is a versatile programming language great for beginners and data science."),
        ).with_metadata("source", "python.org").with_metadata("category", "programming"),

        EmbeddedDocument(
            "doc3",
            "Machine learning uses algorithms to find patterns in data.",
            mock_embedding("Machine learning uses algorithms to find patterns in data."),
        ).with_metadata("source", "wikipedia").with_metadata("category", "ai"),

        EmbeddedDocument(
            "doc4",
            "The weather in San Francisco is often foggy in summer.",
            mock_embedding("The weather in San Francisco is often foggy in summer."),
        ).with_metadata("source", "weather.com").with_metadata("category", "weather"),

        EmbeddedDocument(
            "doc5",
            "Vectors and embeddings are fundamental to semantic search.",
            mock_embedding("Vectors and embeddings are fundamental to semantic search."),
        ).with_metadata("source", "tutorial").with_metadata("category", "ai"),
    ]

    for doc in documents:
        print(f"   Created: {doc.id} (embedding dim: {len(doc.embedding)})")

    # 3. Build a vector index
    print("\n3. Building vector index...")
    index = VectorIndex()

    for doc in documents:
        index.add(doc)

    print(f"   Index contains {len(index)} documents")
    print(f"   Document IDs: {index.ids()}")

    # 4. Perform similarity search
    print("\n4. Searching for similar documents...")

    # Query about programming
    query1 = "What programming languages are good for beginners?"
    query1_embedding = mock_embedding(query1)
    print(f'\n   Query: "{query1}"')

    results = index.search(query1_embedding, 3)
    print("   Top 3 results:")
    for i, result in enumerate(results):
        content_preview = result.document.content[:60]
        print(f"     {i + 1}. [score: {result.score:.3f}] {content_preview}...")

    # Query about AI/ML
    query2 = "How do machine learning algorithms work?"
    query2_embedding = mock_embedding(query2)
    print(f'\n   Query: "{query2}"')

    results = index.search(query2_embedding, 3)
    print("   Top 3 results:")
    for i, result in enumerate(results):
        content_preview = result.document.content[:60]
        print(f"     {i + 1}. [score: {result.score:.3f}] {content_preview}...")

    # 5. Search with threshold
    print("\n5. Searching with minimum score threshold...")

    query3 = "Tell me about Rust programming"
    query3_embedding = mock_embedding(query3)
    print(f'   Query: "{query3}"')
    print("   Minimum score: 0.7")

    results = index.search_with_threshold(query3_embedding, 5, 0.7)
    print(f"   {len(results)} results above threshold:")
    for result in results:
        content_preview = result.document.content[:50]
        print(f"     - [score: {result.score:.3f}] {content_preview}...")

    # 6. Document management
    print("\n6. Document management operations...")

    # Get specific document
    doc = index.get("doc1")
    if doc:
        print(f"   Retrieved doc1: {doc.content[:40]}...")
        print(f"   Metadata: {doc.metadata}")

    # Add a new document
    index.add(EmbeddedDocument(
        "doc6",
        "TypeScript adds static typing to JavaScript.",
        mock_embedding("TypeScript adds static typing to JavaScript."),
    ))
    print(f"   Added doc6, index now has {len(index)} documents")

    # Remove a document
    removed = index.remove("doc4")
    print(f"   Removed doc4: {removed}, index now has {len(index)} documents")

    # 7. Simulate RAG pipeline
    print("\n7. Simulating RAG pipeline...")

    user_question = "What are the benefits of Rust?"
    print(f'   User question: "{user_question}"')

    # Step 1: Retrieve relevant documents
    question_embedding = mock_embedding(user_question)
    retrieved = index.search(question_embedding, 2)

    print("   Retrieved context:")
    context_parts = []
    for result in retrieved:
        print(f"     - {result.document.content}")
        context_parts.append(result.document.content)

    # Step 2: Build augmented prompt (would be sent to LLM)
    context = "\n\n".join(context_parts)
    augmented_prompt = f"""Based on the following context, answer the question.

Context:
{context}

Question: {user_question}

Answer:"""

    print("\n   Augmented prompt (would be sent to LLM):")
    print("   ---")
    for line in augmented_prompt.split('\n')[:8]:
        print(f"   {line}")
    print("   ...")
    print("   ---")

    # 8. Batch operations
    print("\n8. Batch operations...")

    # Create a fresh index with batch add
    batch_docs = [
        EmbeddedDocument("b1", "First batch document", mock_embedding("First batch document")),
        EmbeddedDocument("b2", "Second batch document", mock_embedding("Second batch document")),
        EmbeddedDocument("b3", "Third batch document", mock_embedding("Third batch document")),
    ]

    batch_index = VectorIndex.with_documents(batch_docs)
    print(f"   Created index with {len(batch_index)} documents using with_documents()")

    # Add more in batch
    more_docs = [
        EmbeddedDocument("b4", "Fourth document", mock_embedding("Fourth document")),
        EmbeddedDocument("b5", "Fifth document", mock_embedding("Fifth document")),
    ]
    batch_index.add_batch(more_docs)
    print(f"   After add_batch(): {len(batch_index)} documents")

    # 9. Clear index
    print("\n9. Cleaning up...")
    index.clear()
    print(f"   Index cleared, now contains {len(index)} documents")

    print("\n=== Example Complete ===")


if __name__ == "__main__":
    main()
