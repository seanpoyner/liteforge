#!/usr/bin/env python3
"""
Knowledge example - document storage and search.

Run with: python examples/python/knowledge.py

This example demonstrates:
- LocalKnowledgeBackend for in-memory document storage
- Document creation with metadata
- Text search with SearchOptions
- CRUD operations (Create, Read, Update, Delete)
- Namespace-based organization

Note: Uses LocalKnowledgeBackend which runs entirely in-memory,
no external services or API keys required.
"""

from liteforge import (
    Document,
    LocalKnowledgeBackend,
    SearchOptions,
    ListOptions,
)


def main():
    print("=== LiteForge Knowledge Example (Python) ===\n")

    # 1. Create a local knowledge backend
    print("1. Creating LocalKnowledgeBackend...")
    backend = LocalKnowledgeBackend()
    print("   Created empty in-memory backend")

    # 2. Create and upload documents
    print("\n2. Creating and uploading documents...")

    documents = [
        Document(
            "doc1",
            "Rust is a systems programming language focused on safety, concurrency, and performance.",
            namespace="programming",
        ).with_metadata("language", "rust")
         .with_metadata("category", "systems")
         .with_source("rust-lang.org"),

        Document(
            "doc2",
            "Python is a versatile programming language that excels at scripting, data science, and machine learning.",
            namespace="programming",
        ).with_metadata("language", "python")
         .with_metadata("category", "general-purpose")
         .with_source("python.org"),

        Document(
            "doc3",
            "JavaScript is the language of the web, running in browsers and on servers via Node.js.",
            namespace="programming",
        ).with_metadata("language", "javascript")
         .with_metadata("category", "web")
         .with_source("developer.mozilla.org"),

        Document(
            "doc4",
            "Machine learning is a subset of AI that enables systems to learn from data.",
            namespace="ai",
        ).with_metadata("topic", "ml")
         .with_source("wikipedia.org"),

        Document(
            "doc5",
            "Deep learning uses neural networks with many layers to model complex patterns.",
            namespace="ai",
        ).with_metadata("topic", "deep-learning")
         .with_source("wikipedia.org"),
    ]

    ids = backend.upload(documents)
    print(f"   Uploaded {len(ids)} documents: {ids}")

    # 3. Basic text search
    print("\n3. Searching for documents...")

    results = backend.search("rust programming", SearchOptions().limit(3))

    print("   Query: 'rust programming'")
    print(f"   Found {len(results)} results:")
    for result in results:
        content_preview = result.document.content[:50]
        print(f"     - {result.document.id} (score: {result.score:.3f}): {content_preview}...")

    # 4. Search with namespace filter
    print("\n4. Searching within a namespace...")

    results = backend.search("learning", SearchOptions().namespace("ai"))

    print("   Query: 'learning' in namespace 'ai'")
    print(f"   Found {len(results)} results:")
    for result in results:
        content_preview = result.document.content[:50]
        print(f"     - {result.document.id}: {content_preview}...")

    # 5. Search with metadata filter
    print("\n5. Searching with metadata filter...")

    results = backend.search(
        "programming",
        SearchOptions().filter("language", "python"),
    )

    print("   Query: 'programming' with filter language=python")
    print(f"   Found {len(results)} results:")
    for result in results:
        print(f"     - {result.document.id}")

    # 6. Search with highlights
    print("\n6. Searching with highlights enabled...")

    results = backend.search(
        "neural networks",
        SearchOptions().include_highlights(True),
    )

    print("   Query: 'neural networks'")
    for result in results:
        print(f"   Document: {result.document.id}")
        if result.highlights:
            print("   Highlights:")
            for h in result.highlights:
                print(f'     - "{h}"')

    # 7. Get a specific document
    print("\n7. Retrieving a specific document...")

    doc = backend.get("doc1")
    if doc:
        print(f"   Retrieved: {doc.id}")
        print(f"   Content: {doc.content}")
        print(f"   Namespace: {doc.namespace}")
        print(f"   Metadata: {doc.metadata}")
        print(f"   Source: {doc.source}")

    # 8. List documents with pagination
    print("\n8. Listing documents with pagination...")

    page1 = backend.list(ListOptions().limit(2).offset(0))
    print("   Page 1 (offset 0, limit 2):")
    for doc in page1:
        print(f"     - {doc.id}")

    page2 = backend.list(ListOptions().limit(2).offset(2))
    print("   Page 2 (offset 2, limit 2):")
    for doc in page2:
        print(f"     - {doc.id}")

    # 9. List by namespace
    print("\n9. Listing documents by namespace...")

    ai_docs = backend.list(ListOptions().namespace("ai"))
    print(f"   Documents in 'ai' namespace: {[d.id for d in ai_docs]}")

    # 10. Update a document
    print("\n10. Updating a document...")

    updated_doc = Document(
        "doc1",
        "Rust is a blazingly fast and memory-efficient systems programming language with no runtime or garbage collector.",
        namespace="programming",
    ).with_metadata("language", "rust").with_metadata("category", "systems").with_metadata("updated", True)

    updated = backend.update(updated_doc)
    print(f"   Update successful: {updated}")

    # Verify the update
    doc = backend.get("doc1")
    print(f"   New content: {doc.content}")

    # 11. Get statistics
    print("\n11. Getting knowledge base statistics...")

    stats = backend.stats()
    print(f"   Document count: {stats.document_count}")
    print(f"   Namespace count: {stats.namespace_count}")
    print(f"   Namespaces: {stats.namespaces}")

    # 12. Delete a document
    print("\n12. Deleting a document...")

    deleted = backend.delete("doc5")
    print(f"   Deleted doc5: {deleted}")
    print(f"   New document count: {backend.stats().document_count}")

    # 13. Clear a namespace
    print("\n13. Clearing a namespace...")

    cleared = backend.clear_namespace("ai")
    print(f"   Cleared {cleared} documents from 'ai' namespace")
    print(f"   Remaining documents: {backend.stats().document_count}")

    # 14. Create backend with initial documents
    print("\n14. Creating backend with initial documents...")

    pre_loaded = LocalKnowledgeBackend.with_documents([
        Document("init1", "First preloaded document"),
        Document("init2", "Second preloaded document"),
    ])
    print(f"   Created backend with {pre_loaded.stats().document_count} documents")

    # 15. Clear entire backend
    print("\n15. Clearing entire backend...")

    cleared = backend.clear_all()
    print(f"   Cleared {cleared} documents")
    print(f"   Backend is now empty: {backend.stats().document_count == 0}")

    print("\n=== Example Complete ===")


if __name__ == "__main__":
    main()
