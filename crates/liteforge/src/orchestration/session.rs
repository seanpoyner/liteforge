//! Session management for multi-agent orchestration.
//!
//! Provides session storage and retrieval for maintaining state across interactions.

use super::types::Session;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A session store for managing agent sessions.
#[derive(Debug, Clone)]
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    default_ttl_secs: Option<i64>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore {
    /// Create a new session store.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            default_ttl_secs: None,
        }
    }

    /// Set a default TTL for new sessions.
    pub fn with_default_ttl_secs(mut self, secs: i64) -> Self {
        self.default_ttl_secs = Some(secs);
        self
    }

    /// Create a new session.
    pub async fn create(&self, id: impl Into<String>) -> Session {
        let id = id.into();
        let mut session = Session::new(&id);

        if let Some(ttl) = self.default_ttl_secs {
            session = session.expires_in_secs(ttl);
        }

        let mut sessions = self.sessions.write().await;
        sessions.insert(id, session.clone());
        session
    }

    /// Get a session by ID.
    pub async fn get(&self, id: &str) -> Option<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(id).cloned()
    }

    /// Get a session, returning None if expired.
    pub async fn get_valid(&self, id: &str) -> Option<Session> {
        let session = self.get(id).await?;
        if session.is_expired() {
            self.remove(id).await;
            None
        } else {
            Some(session)
        }
    }

    /// Update a session.
    pub async fn update(&self, session: Session) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.clone(), session);
    }

    /// Remove a session.
    pub async fn remove(&self, id: &str) -> Option<Session> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(id)
    }

    /// Check if a session exists.
    pub async fn exists(&self, id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions.contains_key(id)
    }

    /// Get all session IDs.
    pub async fn list_ids(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// Remove expired sessions.
    pub async fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let expired: Vec<String> = sessions
            .iter()
            .filter(|(_, s)| s.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired.len();
        for id in expired {
            sessions.remove(&id);
        }
        count
    }

    /// Get the number of active sessions.
    pub async fn count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Clear all sessions.
    pub async fn clear(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.clear();
    }
}

/// Get or create a session.
pub async fn get_or_create(store: &SessionStore, id: &str) -> Session {
    if let Some(session) = store.get_valid(id).await {
        session
    } else {
        store.create(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_store_create() {
        let store = SessionStore::new();
        let session = store.create("test-session").await;

        assert_eq!(session.id, "test-session");
        assert!(store.exists("test-session").await);
    }

    #[tokio::test]
    async fn test_session_store_get() {
        let store = SessionStore::new();
        store.create("test-session").await;

        let session = store.get("test-session").await;
        assert!(session.is_some());

        let missing = store.get("nonexistent").await;
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_session_store_update() {
        let store = SessionStore::new();
        let mut session = store.create("test-session").await;

        session.set("key", serde_json::json!("value"));
        store.update(session).await;

        let updated = store.get("test-session").await.unwrap();
        assert_eq!(updated.get("key"), Some(&serde_json::json!("value")));
    }

    #[tokio::test]
    async fn test_session_store_remove() {
        let store = SessionStore::new();
        store.create("test-session").await;

        let removed = store.remove("test-session").await;
        assert!(removed.is_some());
        assert!(!store.exists("test-session").await);
    }

    #[tokio::test]
    async fn test_session_store_with_ttl() {
        let store = SessionStore::new().with_default_ttl_secs(3600);
        let session = store.create("test-session").await;

        assert!(session.expires_at.is_some());
    }

    #[tokio::test]
    async fn test_session_store_list_and_count() {
        let store = SessionStore::new();
        store.create("session1").await;
        store.create("session2").await;
        store.create("session3").await;

        assert_eq!(store.count().await, 3);

        let ids = store.list_ids().await;
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"session1".to_string()));
    }

    #[tokio::test]
    async fn test_session_store_clear() {
        let store = SessionStore::new();
        store.create("session1").await;
        store.create("session2").await;

        store.clear().await;
        assert_eq!(store.count().await, 0);
    }

    #[tokio::test]
    async fn test_get_or_create() {
        let store = SessionStore::new();

        // First call creates
        let session1 = get_or_create(&store, "test-session").await;
        let _ = session1.id.clone(); // use it

        // Second call gets existing
        let session2 = get_or_create(&store, "test-session").await;
        assert_eq!(session2.id, "test-session");

        // Only one session should exist
        assert_eq!(store.count().await, 1);
    }
}
