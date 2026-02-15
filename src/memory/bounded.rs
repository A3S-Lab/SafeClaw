//! Bounded in-memory store with LRU eviction and secure erasure (15.7)
//!
//! Provides a capacity-limited store that evicts the least-recently-used
//! entries when full. Evicted entries have their sensitive fields zeroized
//! before being dropped.
//!
//! **Threat model**: Defends against A1 (malicious user) DoS via unbounded
//! memory growth. See `docs/threat-model.md` §4 AS-4.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use zeroize::Zeroize;

/// Default capacity for bounded stores.
pub const DEFAULT_CAPACITY: usize = 10_000;

/// A capacity-limited in-memory store with LRU eviction.
///
/// When the store reaches capacity, the least-recently-used entry is evicted.
/// Access via `get()` promotes the entry to most-recently-used.
pub struct BoundedStore<T: Clone + HasId + Erasable> {
    entries: Arc<RwLock<BoundedInner<T>>>,
}

/// Internal state for the bounded store.
struct BoundedInner<T> {
    map: HashMap<Uuid, T>,
    /// LRU order: front = oldest, back = newest
    order: VecDeque<Uuid>,
    capacity: usize,
}

/// Trait for types that have a UUID identifier.
pub trait HasId {
    fn id(&self) -> Uuid;
}

/// Trait for types that can securely erase their sensitive fields.
pub trait Erasable {
    /// Zeroize sensitive fields before the value is dropped.
    fn erase(&mut self);
}

impl<T: Clone + HasId + Erasable> BoundedStore<T> {
    /// Create a new bounded store with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(BoundedInner {
                map: HashMap::with_capacity(capacity.min(1024)),
                order: VecDeque::with_capacity(capacity.min(1024)),
                capacity,
            })),
        }
    }

    /// Create a new bounded store with the default capacity.
    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }

    /// Store an entry, evicting the LRU entry if at capacity.
    /// Returns the evicted entry's ID if eviction occurred.
    pub async fn put(&self, entry: T) -> Option<Uuid> {
        let id = entry.id();
        let mut inner = self.entries.write().await;

        // If key already exists, remove from order list (will re-add at back)
        if inner.map.contains_key(&id) {
            inner.order.retain(|k| *k != id);
        }

        // Evict LRU if at capacity
        let evicted = if inner.map.len() >= inner.capacity && !inner.map.contains_key(&id) {
            Self::evict_lru(&mut inner)
        } else {
            None
        };

        inner.map.insert(id, entry);
        inner.order.push_back(id);

        evicted
    }

    /// Retrieve an entry by ID, promoting it to most-recently-used.
    pub async fn get(&self, id: &Uuid) -> Option<T> {
        let mut inner = self.entries.write().await;
        if let Some(entry) = inner.map.get(id) {
            let entry = entry.clone();
            // Promote to MRU
            inner.order.retain(|k| k != id);
            inner.order.push_back(*id);
            Some(entry)
        } else {
            None
        }
    }

    /// Retrieve an entry by ID without promoting it.
    pub async fn peek(&self, id: &Uuid) -> Option<T> {
        self.entries.read().await.map.get(id).cloned()
    }

    /// Remove an entry by ID, securely erasing it.
    pub async fn remove(&self, id: &Uuid) -> Option<T> {
        let mut inner = self.entries.write().await;
        if let Some(mut entry) = inner.map.remove(id) {
            inner.order.retain(|k| k != id);
            entry.erase();
            Some(entry)
        } else {
            None
        }
    }

    /// Get the current number of entries.
    pub async fn len(&self) -> usize {
        self.entries.read().await.map.len()
    }

    /// Check if the store is empty.
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.map.is_empty()
    }

    /// Get the capacity.
    pub async fn capacity(&self) -> usize {
        self.entries.read().await.capacity
    }

    /// Get all entries (no ordering guarantee).
    pub async fn values(&self) -> Vec<T> {
        self.entries.read().await.map.values().cloned().collect()
    }

    /// Clear all entries, securely erasing each one.
    pub async fn clear(&self) {
        let mut inner = self.entries.write().await;
        for (_, entry) in inner.map.iter_mut() {
            entry.erase();
        }
        inner.map.clear();
        inner.order.clear();
    }

    /// Evict the least-recently-used entry, erasing it.
    fn evict_lru(inner: &mut BoundedInner<T>) -> Option<Uuid> {
        if let Some(lru_id) = inner.order.pop_front() {
            if let Some(mut entry) = inner.map.remove(&lru_id) {
                entry.erase();
            }
            Some(lru_id)
        } else {
            None
        }
    }
}

// --- Implement HasId and Erasable for memory types ---

use crate::memory::resource::Resource;
use crate::memory::artifact::Artifact;
use crate::memory::insight::Insight;

impl HasId for Resource {
    fn id(&self) -> Uuid {
        self.id
    }
}

impl Erasable for Resource {
    fn erase(&mut self) {
        self.raw_content.zeroize();
        if let Some(ref mut text) = self.text_content {
            text.zeroize();
        }
        self.user_id.zeroize();
    }
}

impl HasId for Artifact {
    fn id(&self) -> Uuid {
        self.id
    }
}

impl Erasable for Artifact {
    fn erase(&mut self) {
        self.content.zeroize();
    }
}

impl HasId for Insight {
    fn id(&self) -> Uuid {
        self.id
    }
}

impl Erasable for Insight {
    fn erase(&mut self) {
        self.content.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal test type implementing HasId + Erasable + Clone
    #[derive(Debug, Clone)]
    struct TestEntry {
        id: Uuid,
        data: String,
    }

    impl TestEntry {
        fn new(data: &str) -> Self {
            Self {
                id: Uuid::new_v4(),
                data: data.to_string(),
            }
        }
    }

    impl HasId for TestEntry {
        fn id(&self) -> Uuid {
            self.id
        }
    }

    impl Erasable for TestEntry {
        fn erase(&mut self) {
            self.data.zeroize();
        }
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let store = BoundedStore::new(10);
        let entry = TestEntry::new("hello");
        let id = entry.id;

        let evicted = store.put(entry).await;
        assert!(evicted.is_none());

        let retrieved = store.get(&id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data, "hello");
    }

    #[tokio::test]
    async fn test_capacity_eviction() {
        let store = BoundedStore::new(3);

        let e1 = TestEntry::new("first");
        let e2 = TestEntry::new("second");
        let e3 = TestEntry::new("third");
        let e4 = TestEntry::new("fourth");

        let id1 = e1.id;

        store.put(e1).await;
        store.put(e2).await;
        store.put(e3).await;

        assert_eq!(store.len().await, 3);

        // Adding a 4th should evict the LRU (first)
        let evicted = store.put(e4).await;
        assert_eq!(evicted, Some(id1));
        assert_eq!(store.len().await, 3);

        // First entry should be gone
        assert!(store.get(&id1).await.is_none());
    }

    #[tokio::test]
    async fn test_get_promotes_to_mru() {
        let store = BoundedStore::new(3);

        let e1 = TestEntry::new("first");
        let e2 = TestEntry::new("second");
        let e3 = TestEntry::new("third");
        let e4 = TestEntry::new("fourth");

        let id1 = e1.id;
        let id2 = e2.id;

        store.put(e1).await;
        store.put(e2).await;
        store.put(e3).await;

        // Access e1 to promote it to MRU
        store.get(&id1).await;

        // Now e2 is LRU, should be evicted
        let evicted = store.put(e4).await;
        assert_eq!(evicted, Some(id2));

        // e1 should still be present
        assert!(store.get(&id1).await.is_some());
    }

    #[tokio::test]
    async fn test_peek_does_not_promote() {
        let store = BoundedStore::new(3);

        let e1 = TestEntry::new("first");
        let e2 = TestEntry::new("second");
        let e3 = TestEntry::new("third");
        let e4 = TestEntry::new("fourth");

        let id1 = e1.id;

        store.put(e1).await;
        store.put(e2).await;
        store.put(e3).await;

        // Peek at e1 — should NOT promote
        store.peek(&id1).await;

        // e1 is still LRU, should be evicted
        let evicted = store.put(e4).await;
        assert_eq!(evicted, Some(id1));
    }

    #[tokio::test]
    async fn test_remove_erases() {
        let store = BoundedStore::new(10);
        let entry = TestEntry::new("sensitive");
        let id = entry.id;

        store.put(entry).await;
        let removed = store.remove(&id).await;

        assert!(removed.is_some());
        let removed = removed.unwrap();
        // Data should be zeroized (all null bytes → empty after zeroize on String)
        assert!(removed.data.is_empty() || removed.data.bytes().all(|b| b == 0));

        assert!(store.get(&id).await.is_none());
        assert_eq!(store.len().await, 0);
    }

    #[tokio::test]
    async fn test_clear_erases_all() {
        let store = BoundedStore::new(10);
        store.put(TestEntry::new("a")).await;
        store.put(TestEntry::new("b")).await;
        store.put(TestEntry::new("c")).await;

        assert_eq!(store.len().await, 3);

        store.clear().await;

        assert_eq!(store.len().await, 0);
        assert!(store.is_empty().await);
    }

    #[tokio::test]
    async fn test_put_same_id_updates() {
        let store = BoundedStore::new(10);
        let mut entry = TestEntry::new("original");
        let id = entry.id;

        store.put(entry.clone()).await;

        entry.data = "updated".to_string();
        store.put(entry).await;

        // Should still be 1 entry
        assert_eq!(store.len().await, 1);

        let retrieved = store.get(&id).await.unwrap();
        assert_eq!(retrieved.data, "updated");
    }

    #[tokio::test]
    async fn test_values() {
        let store = BoundedStore::new(10);
        store.put(TestEntry::new("a")).await;
        store.put(TestEntry::new("b")).await;

        let values = store.values().await;
        assert_eq!(values.len(), 2);
    }

    #[tokio::test]
    async fn test_default_capacity() {
        let store: BoundedStore<TestEntry> = BoundedStore::with_default_capacity();
        assert_eq!(store.capacity().await, DEFAULT_CAPACITY);
    }

    #[tokio::test]
    async fn test_capacity_one() {
        let store = BoundedStore::new(1);

        let e1 = TestEntry::new("first");
        let e2 = TestEntry::new("second");
        let id1 = e1.id;

        store.put(e1).await;
        let evicted = store.put(e2).await;

        assert_eq!(evicted, Some(id1));
        assert_eq!(store.len().await, 1);
    }

    #[tokio::test]
    async fn test_eviction_order_fifo_without_access() {
        let store = BoundedStore::new(3);

        let entries: Vec<TestEntry> = (0..5).map(|i| TestEntry::new(&format!("entry-{}", i))).collect();
        let ids: Vec<Uuid> = entries.iter().map(|e| e.id).collect();

        for entry in entries {
            store.put(entry).await;
        }

        // Only last 3 should remain
        assert_eq!(store.len().await, 3);
        assert!(store.peek(&ids[0]).await.is_none());
        assert!(store.peek(&ids[1]).await.is_none());
        assert!(store.peek(&ids[2]).await.is_some());
        assert!(store.peek(&ids[3]).await.is_some());
        assert!(store.peek(&ids[4]).await.is_some());
    }

    #[tokio::test]
    async fn test_resource_erasable() {
        use crate::memory::resource::{ContentType, ResourceBuilder, StorageLocation};
        use crate::config::SensitivityLevel;

        let mut resource = ResourceBuilder::new(ContentType::Text)
            .user_id("user-1")
            .channel_id("telegram")
            .chat_id("chat-1")
            .raw_content(b"sensitive data".to_vec())
            .text_content("sensitive text")
            .sensitivity(SensitivityLevel::Sensitive)
            .storage_location(StorageLocation::Memory)
            .build()
            .unwrap();

        resource.erase();

        assert!(resource.raw_content.iter().all(|&b| b == 0));
        assert!(resource.text_content.as_ref().unwrap().is_empty()
            || resource.text_content.as_ref().unwrap().bytes().all(|b| b == 0));
        assert!(resource.user_id.is_empty() || resource.user_id.bytes().all(|b| b == 0));
    }

    #[tokio::test]
    async fn test_artifact_erasable() {
        use crate::memory::artifact::{ArtifactBuilder, ArtifactType};

        let mut artifact = ArtifactBuilder::new(ArtifactType::Entity)
            .content("test@example.com")
            .build()
            .unwrap();

        artifact.erase();

        assert!(artifact.content.is_empty() || artifact.content.bytes().all(|b| b == 0));
    }

    #[tokio::test]
    async fn test_insight_erasable() {
        use crate::memory::insight::{InsightBuilder, InsightType};

        let mut insight = InsightBuilder::new(InsightType::Pattern)
            .content("sensitive pattern")
            .build()
            .unwrap();

        insight.erase();

        assert!(insight.content.is_empty() || insight.content.bytes().all(|b| b == 0));
    }
}
