//! In-memory artifact store for Layer 2 storage
//!
//! Provides async CRUD and query operations for `Artifact` instances using
//! `tokio::sync::RwLock` for concurrent access. Access tracking is built in:
//! `get()` records an access, while `get_without_tracking()` does not.

use super::artifact::{Artifact, ArtifactType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// In-memory store for Layer 2 artifacts
pub struct ArtifactStore {
    artifacts: Arc<RwLock<HashMap<Uuid, Artifact>>>,
}

impl ArtifactStore {
    /// Create a new empty artifact store
    pub fn new() -> Self {
        Self {
            artifacts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store an artifact, returning its ID
    pub async fn put(&self, artifact: Artifact) -> Uuid {
        let id = artifact.id;
        self.artifacts.write().await.insert(id, artifact);
        id
    }

    /// Retrieve an artifact by ID, recording an access (increments access_count
    /// and updates last_accessed).
    pub async fn get(&self, id: &Uuid) -> Option<Artifact> {
        let mut map = self.artifacts.write().await;
        if let Some(artifact) = map.get_mut(id) {
            artifact.record_access();
            Some(artifact.clone())
        } else {
            None
        }
    }

    /// Retrieve an artifact by ID without recording an access.
    pub async fn get_without_tracking(&self, id: &Uuid) -> Option<Artifact> {
        self.artifacts.read().await.get(id).cloned()
    }

    /// Find all artifacts derived from a given resource ID
    pub async fn find_by_resource(&self, resource_id: &Uuid) -> Vec<Artifact> {
        self.artifacts
            .read()
            .await
            .values()
            .filter(|a| a.source_resource_ids.contains(resource_id))
            .cloned()
            .collect()
    }

    /// Find all artifacts of a given type
    pub async fn find_by_type(&self, artifact_type: ArtifactType) -> Vec<Artifact> {
        self.artifacts
            .read()
            .await
            .values()
            .filter(|a| a.artifact_type == artifact_type)
            .cloned()
            .collect()
    }

    /// Find all artifacts with a given tag
    pub async fn find_by_tag(&self, tag: &str) -> Vec<Artifact> {
        self.artifacts
            .read()
            .await
            .values()
            .filter(|a| a.tags.iter().any(|t| t == tag))
            .cloned()
            .collect()
    }

    /// Return the most relevant artifacts, sorted by `relevance_score()`
    /// descending, limited to `limit` results.
    pub async fn find_relevant(&self, limit: usize) -> Vec<Artifact> {
        let map = self.artifacts.read().await;
        let mut all: Vec<Artifact> = map.values().cloned().collect();
        all.sort_by(|a, b| {
            b.relevance_score()
                .partial_cmp(&a.relevance_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all.truncate(limit);
        all
    }

    /// Remove an artifact by ID, returning it if it existed
    pub async fn remove(&self, id: &Uuid) -> Option<Artifact> {
        self.artifacts.write().await.remove(id)
    }
}

impl Default for ArtifactStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SensitivityLevel;
    use crate::memory::artifact::ArtifactBuilder;

    fn build_test_artifact(artifact_type: ArtifactType, content: &str, tags: &[&str]) -> Artifact {
        let mut builder = ArtifactBuilder::new(artifact_type)
            .content(content)
            .sensitivity(SensitivityLevel::Normal)
            .importance(0.5);
        for tag in tags {
            builder = builder.tag(*tag);
        }
        builder.build().unwrap()
    }

    fn build_artifact_from_resource(resource_id: Uuid) -> Artifact {
        ArtifactBuilder::new(ArtifactType::Entity)
            .source_resource(resource_id)
            .content("test@example.com")
            .sensitivity(SensitivityLevel::Sensitive)
            .importance(0.7)
            .tag("email")
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let store = ArtifactStore::new();
        let artifact = build_test_artifact(ArtifactType::Fact, "the sky is blue", &[]);
        let id = artifact.id;

        store.put(artifact).await;

        let retrieved = store.get(&id).await;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.content, "the sky is blue");
    }

    #[tokio::test]
    async fn test_get_tracks_access() {
        let store = ArtifactStore::new();
        let artifact = build_test_artifact(ArtifactType::Entity, "entity-1", &[]);
        let id = artifact.id;

        store.put(artifact).await;

        // First get — should set access_count to 1
        let first = store.get(&id).await.unwrap();
        assert_eq!(first.access_count, 1);
        assert!(first.last_accessed.is_some());

        // Second get — should set access_count to 2
        let second = store.get(&id).await.unwrap();
        assert_eq!(second.access_count, 2);

        // get_without_tracking should NOT increment
        let untracked = store.get_without_tracking(&id).await.unwrap();
        assert_eq!(untracked.access_count, 2);
    }

    #[tokio::test]
    async fn test_find_by_resource() {
        let store = ArtifactStore::new();
        let resource_id = Uuid::new_v4();
        let other_resource_id = Uuid::new_v4();

        store.put(build_artifact_from_resource(resource_id)).await;
        store.put(build_artifact_from_resource(resource_id)).await;
        store
            .put(build_artifact_from_resource(other_resource_id))
            .await;

        let results = store.find_by_resource(&resource_id).await;
        assert_eq!(results.len(), 2);

        let other_results = store.find_by_resource(&other_resource_id).await;
        assert_eq!(other_results.len(), 1);
    }

    #[tokio::test]
    async fn test_find_by_type() {
        let store = ArtifactStore::new();
        store
            .put(build_test_artifact(ArtifactType::Entity, "entity", &[]))
            .await;
        store
            .put(build_test_artifact(ArtifactType::Entity, "entity-2", &[]))
            .await;
        store
            .put(build_test_artifact(ArtifactType::Topic, "topic", &[]))
            .await;

        let entities = store.find_by_type(ArtifactType::Entity).await;
        assert_eq!(entities.len(), 2);

        let topics = store.find_by_type(ArtifactType::Topic).await;
        assert_eq!(topics.len(), 1);
    }

    #[tokio::test]
    async fn test_find_by_tag() {
        let store = ArtifactStore::new();
        store
            .put(build_test_artifact(
                ArtifactType::Entity,
                "email-1",
                &["email"],
            ))
            .await;
        store
            .put(build_test_artifact(
                ArtifactType::Entity,
                "phone-1",
                &["phone"],
            ))
            .await;
        store
            .put(build_test_artifact(
                ArtifactType::Entity,
                "email-2",
                &["email", "contact"],
            ))
            .await;

        let emails = store.find_by_tag("email").await;
        assert_eq!(emails.len(), 2);

        let contacts = store.find_by_tag("contact").await;
        assert_eq!(contacts.len(), 1);

        let empty = store.find_by_tag("nonexistent").await;
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_find_relevant() {
        let store = ArtifactStore::new();

        // Low importance
        let mut low = ArtifactBuilder::new(ArtifactType::Fact)
            .content("low importance")
            .importance(0.1)
            .build()
            .unwrap();
        low.created_at = chrono::Utc::now();

        // High importance
        let mut high = ArtifactBuilder::new(ArtifactType::Fact)
            .content("high importance")
            .importance(0.9)
            .build()
            .unwrap();
        high.created_at = chrono::Utc::now();

        store.put(low).await;
        store.put(high).await;

        let results = store.find_relevant(2).await;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].content, "high importance");
        assert_eq!(results[1].content, "low importance");

        // Limit should be respected
        let limited = store.find_relevant(1).await;
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].content, "high importance");
    }

    #[tokio::test]
    async fn test_remove() {
        let store = ArtifactStore::new();
        let artifact = build_test_artifact(ArtifactType::Entity, "to-delete", &[]);
        let id = artifact.id;

        store.put(artifact).await;
        assert!(store.get_without_tracking(&id).await.is_some());

        let removed = store.remove(&id).await;
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, id);

        assert!(store.get_without_tracking(&id).await.is_none());
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let store = ArtifactStore::new();
        let fake_id = Uuid::new_v4();
        assert!(store.get(&fake_id).await.is_none());
    }
}
