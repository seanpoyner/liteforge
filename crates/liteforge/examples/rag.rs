//! RAG example - vector search and retrieval-augmented generation.
//!
//! Run with: cargo run --example rag
//!
//! This example demonstrates:
//! - Creating and storing embedded documents
//! - Vector similarity functions (cosine, dot product, euclidean)
//! - VectorIndex for similarity search
//! - Building a simple retrieval pipeline
//!
//! Note: Uses mock embeddings to demonstrate functionality without
//! requiring an embedding API. In production, you'd use real embeddings.

use serde_json::json;
use liteforge::rag::{
    cosine_similarity, dot_product, euclidean_distance, normalize, EmbeddedDocument, VectorIndex,
};

/// Generate a simple mock embedding for demonstration.
/// In production, you'd use an embedding model (e.g., text-embedding-3-small).
fn mock_embedding(text: &str) -> Vec<f32> {
    // Simple hash-based mock embedding (for demonstration only)
    let mut embedding = vec![0.0f32; 8];
    for (i, c) in text.chars().enumerate() {
        let idx = i % 8;
        embedding[idx] += (c as u32 as f32) / 1000.0;
    }
    // Normalize to unit length
    normalize(&embedding)
}

fn main() {
    println!("=== LiteForge RAG Example ===\n");

    // 1. Vector math utilities
    println!("1. Vector similarity functions...");

    let v1 = vec![1.0, 0.0, 0.0];
    let v2 = vec![1.0, 0.0, 0.0]; // Same direction
    let v3 = vec![0.0, 1.0, 0.0]; // Orthogonal
    let v4 = vec![0.707, 0.707, 0.0]; // 45 degrees

    println!("   Vectors:");
    println!("     v1 = {:?}", v1);
    println!("     v2 = {:?}", v2);
    println!("     v3 = {:?}", v3);
    println!("     v4 = {:?}", v4);

    println!("\n   Cosine similarity:");
    println!("     v1 · v2 (same) = {:.3}", cosine_similarity(&v1, &v2));
    println!(
        "     v1 · v3 (orthogonal) = {:.3}",
        cosine_similarity(&v1, &v3)
    );
    println!("     v1 · v4 (45°) = {:.3}", cosine_similarity(&v1, &v4));

    println!("\n   Dot product:");
    println!("     v1 · v2 = {:.3}", dot_product(&v1, &v2));
    println!("     v1 · v4 = {:.3}", dot_product(&v1, &v4));

    println!("\n   Euclidean distance:");
    println!("     v1 ↔ v2 = {:.3}", euclidean_distance(&v1, &v2));
    println!("     v1 ↔ v3 = {:.3}", euclidean_distance(&v1, &v3));

    println!("\n   Normalization:");
    let unnorm = vec![3.0, 4.0];
    let normalized = normalize(&unnorm);
    println!("     {:?} → {:?} (length 1.0)", unnorm, normalized);

    // 2. Create embedded documents
    println!("\n2. Creating embedded documents...");

    let documents = vec![
        EmbeddedDocument::new(
            "doc1",
            "Rust is a systems programming language focused on safety and performance.",
            mock_embedding(
                "Rust is a systems programming language focused on safety and performance.",
            ),
        )
        .metadata("source", json!("rust-lang.org"))
        .metadata("category", json!("programming")),
        EmbeddedDocument::new(
            "doc2",
            "Python is a versatile programming language great for beginners and data science.",
            mock_embedding(
                "Python is a versatile programming language great for beginners and data science.",
            ),
        )
        .metadata("source", json!("python.org"))
        .metadata("category", json!("programming")),
        EmbeddedDocument::new(
            "doc3",
            "Machine learning uses algorithms to find patterns in data.",
            mock_embedding("Machine learning uses algorithms to find patterns in data."),
        )
        .metadata("source", json!("wikipedia"))
        .metadata("category", json!("ai")),
        EmbeddedDocument::new(
            "doc4",
            "The weather in San Francisco is often foggy in summer.",
            mock_embedding("The weather in San Francisco is often foggy in summer."),
        )
        .metadata("source", json!("weather.com"))
        .metadata("category", json!("weather")),
        EmbeddedDocument::new(
            "doc5",
            "Vectors and embeddings are fundamental to semantic search.",
            mock_embedding("Vectors and embeddings are fundamental to semantic search."),
        )
        .metadata("source", json!("tutorial"))
        .metadata("category", json!("ai")),
    ];

    for doc in &documents {
        println!(
            "   Created: {} (embedding dim: {})",
            doc.id,
            doc.embedding.len()
        );
    }

    // 3. Build a vector index
    println!("\n3. Building vector index...");
    let mut index = VectorIndex::new();

    for doc in documents {
        index.add(doc);
    }

    println!("   Index contains {} documents", index.len());
    println!("   Document IDs: {:?}", index.ids());

    // 4. Perform similarity search
    println!("\n4. Searching for similar documents...");

    // Query about programming
    let query1 = "What programming languages are good for beginners?";
    let query1_embedding = mock_embedding(query1);
    println!("\n   Query: \"{}\"", query1);

    let results = index.search(&query1_embedding, 3);
    println!("   Top 3 results:");
    for (i, result) in results.iter().enumerate() {
        println!(
            "     {}. [score: {:.3}] {}",
            i + 1,
            result.score,
            result.document.content.chars().take(60).collect::<String>()
        );
    }

    // Query about AI/ML
    let query2 = "How do machine learning algorithms work?";
    let query2_embedding = mock_embedding(query2);
    println!("\n   Query: \"{}\"", query2);

    let results = index.search(&query2_embedding, 3);
    println!("   Top 3 results:");
    for (i, result) in results.iter().enumerate() {
        println!(
            "     {}. [score: {:.3}] {}",
            i + 1,
            result.score,
            result.document.content.chars().take(60).collect::<String>()
        );
    }

    // 5. Search with threshold
    println!("\n5. Searching with minimum score threshold...");

    let query3 = "Tell me about Rust programming";
    let query3_embedding = mock_embedding(query3);
    println!("   Query: \"{}\"", query3);
    println!("   Minimum score: 0.7");

    let results = index.search_with_threshold(&query3_embedding, 5, 0.7);
    println!("   {} results above threshold:", results.len());
    for result in &results {
        println!(
            "     - [score: {:.3}] {}",
            result.score,
            result.document.content.chars().take(50).collect::<String>()
        );
    }

    // 6. Document management
    println!("\n6. Document management operations...");

    // Get specific document
    if let Some(doc) = index.get("doc1") {
        println!(
            "   Retrieved doc1: {:?}",
            doc.content.chars().take(40).collect::<String>()
        );
        println!("   Metadata: {:?}", doc.metadata);
    }

    // Add a new document
    index.add(EmbeddedDocument::new(
        "doc6",
        "TypeScript adds static typing to JavaScript.",
        mock_embedding("TypeScript adds static typing to JavaScript."),
    ));
    println!("   Added doc6, index now has {} documents", index.len());

    // Remove a document
    let removed = index.remove("doc4");
    println!(
        "   Removed doc4: {}, index now has {} documents",
        removed,
        index.len()
    );

    // 7. Simulate RAG pipeline
    println!("\n7. Simulating RAG pipeline...");

    let user_question = "What are the benefits of Rust?";
    println!("   User question: \"{}\"", user_question);

    // Step 1: Retrieve relevant documents
    let question_embedding = mock_embedding(user_question);
    let retrieved = index.search(&question_embedding, 2);

    println!("   Retrieved context:");
    let mut context_parts = Vec::new();
    for result in &retrieved {
        println!("     - {}", result.document.content);
        context_parts.push(result.document.content.clone());
    }

    // Step 2: Build augmented prompt (would be sent to LLM)
    let context = context_parts.join("\n\n");
    let augmented_prompt = format!(
        "Based on the following context, answer the question.\n\n\
         Context:\n{}\n\n\
         Question: {}\n\n\
         Answer:",
        context, user_question
    );

    println!("\n   Augmented prompt (would be sent to LLM):");
    println!("   ---");
    for line in augmented_prompt.lines().take(8) {
        println!("   {}", line);
    }
    println!("   ...");
    println!("   ---");

    // 8. Clear index
    println!("\n8. Cleaning up...");
    index.clear();
    println!("   Index cleared, now contains {} documents", index.len());

    println!("\n=== Example Complete ===");
}
