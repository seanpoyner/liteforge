//! Integration tests for the refactored SDK.
//!
//! These tests verify that the single-dispatch transport refactoring
//! works correctly with actual API calls.
//!
//! Requires LITEFORGE_API_KEY environment variable to be set in .env

#[cfg(test)]
mod integration_tests {
    use std::path::PathBuf;
    use std::sync::Once;
    use liteforge::{AsyncForgeClient, Message, ForgeClient};

    static INIT: Once = Once::new();

    fn init() {
        INIT.call_once(|| {
            // Try to load .env from workspace root
            // First try current directory, then parent directories
            let _ = dotenvy::dotenv();

            // Also try explicit path to workspace root
            if std::env::var("LITEFORGE_API_KEY").is_err() {
                let workspace_root = PathBuf::from("/mnt/c/users/sbpoy839/.projects/liteforge/.env");
                if workspace_root.exists() {
                    let _ = dotenvy::from_path(&workspace_root);
                }
            }
        });
    }

    #[tokio::test]
    async fn test_async_client() {
        init();
        // This now delegates to transport::request_with_body internally
        let client = AsyncForgeClient::new();
        let response = client.complete(vec![Message::user("Hello")]).await;
        if let Err(e) = &response {
            eprintln!("Error: {}", e);
        }
        assert!(
            response.is_ok(),
            "Async client should complete successfully"
        );
    }

    #[test]
    fn test_sync_client() {
        init();
        // This now uses the stored Runtime instead of creating one per call
        let client = ForgeClient::new();
        let response = client.complete(vec![Message::user("Hello")]);
        if let Err(e) = &response {
            eprintln!("Error: {}", e);
        }
        assert!(response.is_ok(), "Sync client should complete successfully");
    }

    #[tokio::test]
    async fn test_async_client_with_model_override() {
        init();
        let client = AsyncForgeClient::new();
        let response = client
            .complete_with_model(
                "anthropic.claude-haiku-4-5-20251001-v1:0",
                vec![Message::user("What is 2+2?")],
            )
            .await;
        if let Err(e) = &response {
            eprintln!("Error: {}", e);
        }
        assert!(
            response.is_ok(),
            "Async client with model override should work"
        );
    }

    #[test]
    fn test_sync_client_with_model_override() {
        init();
        let client = ForgeClient::new();
        let response = client.complete_with_model(
            "anthropic.claude-haiku-4-5-20251001-v1:0",
            vec![Message::user("What is 2+2?")],
        );
        if let Err(e) = &response {
            eprintln!("Error: {}", e);
        }
        assert!(
            response.is_ok(),
            "Sync client with model override should work"
        );
    }

    #[tokio::test]
    async fn test_list_models() {
        init();
        // Tests the request_no_body dispatch path
        let client = AsyncForgeClient::new();
        let response = client.list_models().await;
        if let Err(e) = &response {
            eprintln!("Error: {}", e);
        }
        assert!(response.is_ok(), "List models should succeed");

        if let Ok(models) = response {
            assert!(!models.data.is_empty(), "Should return at least one model");
        }
    }

    #[test]
    fn test_list_models_sync() {
        init();
        // Tests the sync wrapper around async list_models
        let client = ForgeClient::new();
        let response = client.list_models();
        if let Err(e) = &response {
            eprintln!("Error: {}", e);
        }
        assert!(response.is_ok(), "List models (sync) should succeed");

        if let Ok(models) = response {
            assert!(!models.data.is_empty(), "Should return at least one model");
        }
    }
}
