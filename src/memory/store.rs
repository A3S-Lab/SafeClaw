//! In-memory resource store for Layer 1 storage
//!
//! Provides async CRUD operations for `Resource` instances using
//! `tokio::sync::RwLock` for concurrent access.

use super::resource::Resource;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// In-memory store for Layer 1 resources
pub struct ResourceStore {
    resources: Arc<RwLock<HashMap<Uuid, Resource>>>,
}

impl ResourceStore {
    /// Create a new empty resource store
    pub fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a resource, returning its ID
    pub async fn put(&self, resource: Resource) -> Uuid {
        let id = resource.id;
        self.resources.write().await.insert(id, resource);
        id
    }

    /// Retrieve a resource by ID
    pub async fn get(&self, id: &Uuid) -> Option<Resource> {
        self.resources.read().await.get(id).cloned()
    }

    /// List all resources belonging to a user
    pub async fn list_by_user(&self, user_id: &str) -> Vec<Resource> {
        self.resources
            .read()
            .await
            .values()
            .filter(|r| r.user_id == user_id)
            .cloned()
            .collect()
    }

    /// List all resources for a specific channel
    pub async fn list_by_channel(&self, channel_id: &str) -> Vec<Resource> {
        self.resources
            .read()
            .await
            .values()
            .filter(|r| r.channel_id == channel_id)
            .cloned()
            .collect()
    }

    /// Remove a resource by ID, returning it if it existed
    pub async fn remove(&self, id: &Uuid) -> Option<Resource> {
        self.resources.write().await.remove(id)
    }
}

impl Default for ResourceStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SensitivityLevel;
    use crate::memory::resource::{ContentType, ResourceBuilder, StorageLocation};

    fn build_test_resource(user_id: &str, channel_id: &str) -> Resource {
        ResourceBuilder::new(ContentType::Text)
            .user_id(user_id)
            .channel_id(channel_id)
            .chat_id("chat-1")
            .text_content("test content")
            .raw_content(b"test content".to_vec())
            .sensitivity(SensitivityLevel::Normal)
            .storage_location(StorageLocation::Memory)
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let store = ResourceStore::new();
        let resource = build_test_resource("user-1", "telegram");
        let id = resource.id;

        store.put(resource).await;

        let retrieved = store.get(&id).await;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.user_id, "user-1");
    }

    #[tokio::test]
    async fn test_list_by_user() {
        let store = ResourceStore::new();

        store.put(build_test_resource("user-1", "telegram")).await;
        store.put(build_test_resource("user-1", "slack")).await;
        store.put(build_test_resource("user-2", "telegram")).await;

        let user1_resources = store.list_by_user("user-1").await;
        assert_eq!(user1_resources.len(), 2);

        let user2_resources = store.list_by_user("user-2").await;
        assert_eq!(user2_resources.len(), 1);
    }

    #[tokio::test]
    async fn test_list_by_channel() {
        let store = ResourceStore::new();

        store.put(build_test_resource("user-1", "telegram")).await;
        store.put(build_test_resource("user-2", "telegram")).await;
        store.put(build_test_resource("user-1", "slack")).await;

        let telegram = store.list_by_channel("telegram").await;
        assert_eq!(telegram.len(), 2);

        let slack = store.list_by_channel("slack").await;
        assert_eq!(slack.len(), 1);
    }

    #[tokio::test]
    async fn test_remove() {
        let store = ResourceStore::new();
        let resource = build_test_resource("user-1", "telegram");
        let id = resource.id;

        store.put(resource).await;
        assert!(store.get(&id).await.is_some());

        let removed = store.remove(&id).await;
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, id);

        assert!(store.get(&id).await.is_none());
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let store = ResourceStore::new();
        let fake_id = Uuid::new_v4();
        assert!(store.get(&fake_id).await.is_none());
    }
}
