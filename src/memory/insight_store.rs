//! In-memory insight store for Layer 3 storage
//!
//! Provides async CRUD and query operations for `Insight` instances using
//! `tokio::sync::RwLock` for concurrent access. Access tracking is built in:
//! `get()` records an access, while `get_without_tracking()` does not.

use super::insight::{Insight, InsightType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// In-memory store for Layer 3 insights
pub struct InsightStore {
    insights: Arc<RwLock<HashMap<Uuid, Insight>>>,
}

impl InsightStore {
    /// Create a new empty insight store
    pub fn new() -> Self {
        Self {
            insights: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store an insight, returning its ID
    pub async fn put(&self, insight: Insight) -> Uuid {
        let id = insight.id;
        self.insights.write().await.insert(id, insight);
        id
    }

    /// Retrieve an insight by ID, recording an access (increments access_count
    /// and updates last_accessed).
    pub async fn get(&self, id: &Uuid) -> Option<Insight> {
        let mut map = self.insights.write().await;
        if let Some(insight) = map.get_mut(id) {
            insight.record_access();
            Some(insight.clone())
        } else {
            None
        }
    }

    /// Retrieve an insight by ID without recording an access.
    pub async fn get_without_tracking(&self, id: &Uuid) -> Option<Insight> {
        self.insights.read().await.get(id).cloned()
    }

    /// Find all insights derived from a given artifact ID
    pub async fn find_by_artifact(&self, artifact_id: &Uuid) -> Vec<Insight> {
        self.insights
            .read()
            .await
            .values()
            .filter(|i| i.source_artifact_ids.contains(artifact_id))
            .cloned()
            .collect()
    }

    /// Find all insights of a given type
    pub async fn find_by_type(&self, insight_type: InsightType) -> Vec<Insight> {
        self.insights
            .read()
            .await
            .values()
            .filter(|i| i.insight_type == insight_type)
            .cloned()
            .collect()
    }

    /// Find all insights with a given tag
    pub async fn find_by_tag(&self, tag: &str) -> Vec<Insight> {
        self.insights
            .read()
            .await
            .values()
            .filter(|i| i.tags.iter().any(|t| t == tag))
            .cloned()
            .collect()
    }

    /// Return the most relevant insights, sorted by `relevance_score()`
    /// descending, limited to `limit` results.
    pub async fn find_relevant(&self, limit: usize) -> Vec<Insight> {
        let map = self.insights.read().await;
        let mut all: Vec<Insight> = map.values().cloned().collect();
        all.sort_by(|a, b| {
            b.relevance_score()
                .partial_cmp(&a.relevance_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all.truncate(limit);
        all
    }

    /// Find all insights with confidence at or above the given threshold
    pub async fn find_by_confidence(&self, threshold: f32) -> Vec<Insight> {
        self.insights
            .read()
            .await
            .values()
            .filter(|i| i.confidence >= threshold)
            .cloned()
            .collect()
    }

    /// Remove an insight by ID, returning it if it existed
    pub async fn remove(&self, id: &Uuid) -> Option<Insight> {
        self.insights.write().await.remove(id)
    }
}

impl Default for InsightStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SensitivityLevel;
    use crate::memory::insight::InsightBuilder;

    fn build_test_insight(
        insight_type: InsightType,
        content: &str,
        tags: &[&str],
    ) -> Insight {
        let mut builder = InsightBuilder::new(insight_type)
            .content(content)
            .sensitivity(SensitivityLevel::Normal)
            .importance(0.5)
            .confidence(0.7);
        for tag in tags {
            builder = builder.tag(*tag);
        }
        builder.build().unwrap()
    }

    fn build_insight_from_artifact(artifact_id: Uuid) -> Insight {
        InsightBuilder::new(InsightType::Pattern)
            .source_artifact(artifact_id)
            .content("pattern from artifact")
            .sensitivity(SensitivityLevel::Sensitive)
            .importance(0.7)
            .confidence(0.8)
            .tag("entity_frequency")
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let store = InsightStore::new();
        let insight = build_test_insight(InsightType::Pattern, "a pattern", &[]);
        let id = insight.id;

        store.put(insight).await;

        let retrieved = store.get(&id).await;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.content, "a pattern");
    }

    #[tokio::test]
    async fn test_get_tracks_access() {
        let store = InsightStore::new();
        let insight = build_test_insight(InsightType::Pattern, "tracked", &[]);
        let id = insight.id;

        store.put(insight).await;

        let first = store.get(&id).await.unwrap();
        assert_eq!(first.access_count, 1);
        assert!(first.last_accessed.is_some());

        let second = store.get(&id).await.unwrap();
        assert_eq!(second.access_count, 2);

        let untracked = store.get_without_tracking(&id).await.unwrap();
        assert_eq!(untracked.access_count, 2);
    }

    #[tokio::test]
    async fn test_find_by_artifact() {
        let store = InsightStore::new();
        let artifact_id = Uuid::new_v4();
        let other_artifact_id = Uuid::new_v4();

        store.put(build_insight_from_artifact(artifact_id)).await;
        store.put(build_insight_from_artifact(artifact_id)).await;
        store
            .put(build_insight_from_artifact(other_artifact_id))
            .await;

        let results = store.find_by_artifact(&artifact_id).await;
        assert_eq!(results.len(), 2);

        let other_results = store.find_by_artifact(&other_artifact_id).await;
        assert_eq!(other_results.len(), 1);
    }

    #[tokio::test]
    async fn test_find_by_type() {
        let store = InsightStore::new();
        store
            .put(build_test_insight(InsightType::Pattern, "pattern-1", &[]))
            .await;
        store
            .put(build_test_insight(InsightType::Pattern, "pattern-2", &[]))
            .await;
        store
            .put(build_test_insight(InsightType::Summary, "summary-1", &[]))
            .await;

        let patterns = store.find_by_type(InsightType::Pattern).await;
        assert_eq!(patterns.len(), 2);

        let summaries = store.find_by_type(InsightType::Summary).await;
        assert_eq!(summaries.len(), 1);
    }

    #[tokio::test]
    async fn test_find_by_tag() {
        let store = InsightStore::new();
        store
            .put(build_test_insight(
                InsightType::Pattern,
                "pattern-1",
                &["entity_frequency"],
            ))
            .await;
        store
            .put(build_test_insight(
                InsightType::Summary,
                "summary-1",
                &["topic_aggregation"],
            ))
            .await;
        store
            .put(build_test_insight(
                InsightType::Pattern,
                "pattern-2",
                &["entity_frequency", "email"],
            ))
            .await;

        let entity_freq = store.find_by_tag("entity_frequency").await;
        assert_eq!(entity_freq.len(), 2);

        let topic_agg = store.find_by_tag("topic_aggregation").await;
        assert_eq!(topic_agg.len(), 1);

        let empty = store.find_by_tag("nonexistent").await;
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_find_relevant() {
        let store = InsightStore::new();

        let mut low = InsightBuilder::new(InsightType::Pattern)
            .content("low importance")
            .importance(0.1)
            .confidence(0.5)
            .build()
            .unwrap();
        low.created_at = chrono::Utc::now();

        let mut high = InsightBuilder::new(InsightType::Pattern)
            .content("high importance")
            .importance(0.9)
            .confidence(0.9)
            .build()
            .unwrap();
        high.created_at = chrono::Utc::now();

        store.put(low).await;
        store.put(high).await;

        let results = store.find_relevant(2).await;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].content, "high importance");
        assert_eq!(results[1].content, "low importance");

        let limited = store.find_relevant(1).await;
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].content, "high importance");
    }

    #[tokio::test]
    async fn test_find_by_confidence() {
        let store = InsightStore::new();

        let low_conf = InsightBuilder::new(InsightType::Pattern)
            .content("low confidence")
            .confidence(0.3)
            .build()
            .unwrap();
        let high_conf = InsightBuilder::new(InsightType::Pattern)
            .content("high confidence")
            .confidence(0.9)
            .build()
            .unwrap();

        store.put(low_conf).await;
        store.put(high_conf).await;

        let above_half = store.find_by_confidence(0.5).await;
        assert_eq!(above_half.len(), 1);
        assert_eq!(above_half[0].content, "high confidence");

        let above_zero = store.find_by_confidence(0.0).await;
        assert_eq!(above_zero.len(), 2);
    }

    #[tokio::test]
    async fn test_remove() {
        let store = InsightStore::new();
        let insight = build_test_insight(InsightType::Pattern, "to-delete", &[]);
        let id = insight.id;

        store.put(insight).await;
        assert!(store.get_without_tracking(&id).await.is_some());

        let removed = store.remove(&id).await;
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, id);

        assert!(store.get_without_tracking(&id).await.is_none());
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let store = InsightStore::new();
        let fake_id = Uuid::new_v4();
        assert!(store.get(&fake_id).await.is_none());
    }

    #[tokio::test]
    async fn test_empty_store_queries() {
        let store = InsightStore::new();

        assert!(store.find_by_artifact(&Uuid::new_v4()).await.is_empty());
        assert!(store.find_by_type(InsightType::Pattern).await.is_empty());
        assert!(store.find_by_tag("anything").await.is_empty());
        assert!(store.find_relevant(10).await.is_empty());
        assert!(store.find_by_confidence(0.0).await.is_empty());
    }

    #[tokio::test]
    async fn test_put_overwrites_same_id() {
        let store = InsightStore::new();
        let mut insight = build_test_insight(InsightType::Pattern, "original", &[]);
        let id = insight.id;

        store.put(insight.clone()).await;

        // Mutate content and re-put with same ID
        insight.content = "updated".to_string();
        store.put(insight).await;

        let retrieved = store.get_without_tracking(&id).await.unwrap();
        assert_eq!(retrieved.content, "updated");
    }

    #[tokio::test]
    async fn test_find_by_artifact_no_match() {
        let store = InsightStore::new();
        let artifact_id = Uuid::new_v4();
        let unrelated_artifact_id = Uuid::new_v4();

        store.put(build_insight_from_artifact(artifact_id)).await;

        let results = store.find_by_artifact(&unrelated_artifact_id).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_find_by_confidence_exact_threshold() {
        let store = InsightStore::new();

        let exact = InsightBuilder::new(InsightType::Pattern)
            .content("exactly at threshold")
            .confidence(0.5)
            .build()
            .unwrap();
        let below = InsightBuilder::new(InsightType::Pattern)
            .content("below threshold")
            .confidence(0.49)
            .build()
            .unwrap();

        store.put(exact).await;
        store.put(below).await;

        // Exact match should be included (>=), just below should be excluded
        let results = store.find_by_confidence(0.5).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "exactly at threshold");
    }

    #[tokio::test]
    async fn test_remove_nonexistent() {
        let store = InsightStore::new();
        let result = store.remove(&Uuid::new_v4()).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_artifact_multi_source() {
        // An insight with multiple source artifact IDs should be found by any of them
        let store = InsightStore::new();
        let art_a = Uuid::new_v4();
        let art_b = Uuid::new_v4();

        let insight = InsightBuilder::new(InsightType::Correlation)
            .source_artifact(art_a)
            .source_artifact(art_b)
            .content("correlation between a and b")
            .confidence(0.6)
            .build()
            .unwrap();

        store.put(insight).await;

        let by_a = store.find_by_artifact(&art_a).await;
        assert_eq!(by_a.len(), 1);

        let by_b = store.find_by_artifact(&art_b).await;
        assert_eq!(by_b.len(), 1);

        assert_eq!(by_a[0].id, by_b[0].id);
    }

    #[tokio::test]
    async fn test_find_relevant_empty_store() {
        let store = InsightStore::new();
        let results = store.find_relevant(5).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_find_by_type_all_variants() {
        let store = InsightStore::new();
        store.put(build_test_insight(InsightType::Pattern, "p", &[])).await;
        store.put(build_test_insight(InsightType::Summary, "s", &[])).await;
        store.put(build_test_insight(InsightType::Correlation, "c", &[])).await;
        store.put(build_test_insight(InsightType::Trend, "t", &[])).await;

        assert_eq!(store.find_by_type(InsightType::Pattern).await.len(), 1);
        assert_eq!(store.find_by_type(InsightType::Summary).await.len(), 1);
        assert_eq!(store.find_by_type(InsightType::Correlation).await.len(), 1);
        assert_eq!(store.find_by_type(InsightType::Trend).await.len(), 1);
    }

    #[tokio::test]
    async fn test_default_creates_empty_store() {
        let store = InsightStore::default();
        assert!(store.find_relevant(10).await.is_empty());
    }
}
