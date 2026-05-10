//! Knowledge example - document storage and search.
//!
//! Run with: cargo run --example knowledge
//!
//! This example demonstrates:
//! - LocalKnowledgeBackend for in-memory document storage
//! - Document creation with metadata
//! - Text search with SearchOptions
//! - CRUD operations (Create, Read, Update, Delete)
//! - Namespace-based organization
//!
//! Note: Uses LocalKnowledgeBackend which runs entirely in-memory,
//! no external services or API keys required.

use serde_json::json;
use liteforge::knowledge::{
    Document, KnowledgeClient, ListOptions, LocalKnowledgeBackend, SearchOptions,
};

#[tokio::main]
async fn main() {
    println!("=== LiteForge Knowledge Example ===\n");

    // 1. Create a local knowledge backend
    println!("1. Creating LocalKnowledgeBackend...");
    let backend = LocalKnowledgeBackend::new();
    println!("   Created empty in-memory backend");

    // 2. Create and upload documents
    println!("\n2. Creating and uploading documents...");

    let documents = vec![
        Document::new("doc1", "Rust is a systems programming language focused on safety, concurrency, and performance.")
            .namespace("programming")
            .metadata("language", json!("rust"))
            .metadata("category", json!("systems"))
            .source("rust-lang.org"),

        Document::new("doc2", "Python is a versatile programming language that excels at scripting, data science, and machine learning.")
            .namespace("programming")
            .metadata("language", json!("python"))
            .metadata("category", json!("general-purpose"))
            .source("python.org"),

        Document::new("doc3", "JavaScript is the language of the web, running in browsers and on servers via Node.js.")
            .namespace("programming")
            .metadata("language", json!("javascript"))
            .metadata("category", json!("web"))
            .source("developer.mozilla.org"),

        Document::new("doc4", "Machine learning is a subset of AI that enables systems to learn from data.")
            .namespace("ai")
            .metadata("topic", json!("ml"))
            .source("wikipedia.org"),

        Document::new("doc5", "Deep learning uses neural networks with many layers to model complex patterns.")
            .namespace("ai")
            .metadata("topic", json!("deep-learning"))
            .source("wikipedia.org"),
    ];

    let ids = backend.upload(documents).await.unwrap();
    println!("   Uploaded {} documents: {:?}", ids.len(), ids);

    // 3. Basic text search
    println!("\n3. Searching for documents...");

    let results = backend
        .search("rust programming", SearchOptions::new().limit(3))
        .await
        .unwrap();

    println!("   Query: 'rust programming'");
    println!("   Found {} results:", results.len());
    for result in &results {
        println!(
            "     - {} (score: {:.3}): {}",
            result.document.id,
            result.score,
            result.document.content.chars().take(50).collect::<String>()
        );
    }

    // 4. Search with namespace filter
    println!("\n4. Searching within a namespace...");

    let results = backend
        .search("learning", SearchOptions::new().namespace("ai"))
        .await
        .unwrap();

    println!("   Query: 'learning' in namespace 'ai'");
    println!("   Found {} results:", results.len());
    for result in &results {
        println!("     - {}: {}", result.document.id, result.document.content.chars().take(50).collect::<String>());
    }

    // 5. Search with metadata filter
    println!("\n5. Searching with metadata filter...");

    let results = backend
        .search(
            "programming",
            SearchOptions::new().filter("language", json!("python")),
        )
        .await
        .unwrap();

    println!("   Query: 'programming' with filter language=python");
    println!("   Found {} results:", results.len());
    for result in &results {
        println!("     - {}", result.document.id);
    }

    // 6. Search with highlights
    println!("\n6. Searching with highlights enabled...");

    let results = backend
        .search(
            "neural networks",
            SearchOptions::new().include_highlights(true),
        )
        .await
        .unwrap();

    println!("   Query: 'neural networks'");
    for result in &results {
        println!("   Document: {}", result.document.id);
        if !result.highlights.is_empty() {
            println!("   Highlights:");
            for h in &result.highlights {
                println!("     - \"{}\"", h);
            }
        }
    }

    // 7. Get a specific document
    println!("\n7. Retrieving a specific document...");

    if let Some(doc) = backend.get("doc1").await.unwrap() {
        println!("   Retrieved: {}", doc.id);
        println!("   Content: {}", doc.content);
        println!("   Namespace: {:?}", doc.namespace);
        println!("   Metadata: {:?}", doc.metadata);
        println!("   Source: {:?}", doc.source);
    }

    // 8. List documents with pagination
    println!("\n8. Listing documents with pagination...");

    let page1 = backend
        .list(ListOptions::new().limit(2).offset(0))
        .await
        .unwrap();
    println!("   Page 1 (offset 0, limit 2):");
    for doc in &page1 {
        println!("     - {}", doc.id);
    }

    let page2 = backend
        .list(ListOptions::new().limit(2).offset(2))
        .await
        .unwrap();
    println!("   Page 2 (offset 2, limit 2):");
    for doc in &page2 {
        println!("     - {}", doc.id);
    }

    // 9. List by namespace
    println!("\n9. Listing documents by namespace...");

    let ai_docs = backend
        .list(ListOptions::new().namespace("ai"))
        .await
        .unwrap();
    println!("   Documents in 'ai' namespace: {:?}", ai_docs.iter().map(|d| &d.id).collect::<Vec<_>>());

    // 10. Update a document
    println!("\n10. Updating a document...");

    let updated_doc = Document::new("doc1", "Rust is a blazingly fast and memory-efficient systems programming language with no runtime or garbage collector.")
        .namespace("programming")
        .metadata("language", json!("rust"))
        .metadata("category", json!("systems"))
        .metadata("updated", json!(true));

    let updated = backend.update(updated_doc).await.unwrap();
    println!("   Update successful: {}", updated);

    // Verify the update
    let doc = backend.get("doc1").await.unwrap().unwrap();
    println!("   New content: {}", doc.content);

    // 11. Get statistics
    println!("\n11. Getting knowledge base statistics...");

    let stats = backend.stats().await.unwrap();
    println!("   Document count: {}", stats.document_count);
    println!("   Namespace count: {}", stats.namespace_count);
    println!("   Namespaces: {:?}", stats.namespaces);

    // 12. Delete a document
    println!("\n12. Deleting a document...");

    let deleted = backend.delete("doc5").await.unwrap();
    println!("   Deleted doc5: {}", deleted);
    println!("   New document count: {}", backend.stats().await.unwrap().document_count);

    // 13. Clear a namespace
    println!("\n13. Clearing a namespace...");

    let cleared = backend.clear(Some("ai")).await.unwrap();
    println!("   Cleared {} documents from 'ai' namespace", cleared);
    println!("   Remaining documents: {}", backend.stats().await.unwrap().document_count);

    // 14. Create backend with initial documents
    println!("\n14. Creating backend with initial documents...");

    let pre_loaded = LocalKnowledgeBackend::with_documents(vec![
        Document::new("init1", "First preloaded document"),
        Document::new("init2", "Second preloaded document"),
    ]);
    println!("   Created backend with {} documents", pre_loaded.stats().await.unwrap().document_count);

    // 15. Clear entire backend
    println!("\n15. Clearing entire backend...");

    let cleared = backend.clear(None).await.unwrap();
    println!("   Cleared {} documents", cleared);
    println!("   Backend is now empty: {}", backend.stats().await.unwrap().document_count == 0);

    println!("\n=== Example Complete ===");
}
