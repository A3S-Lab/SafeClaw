//! Layer 3 insight data types
//!
//! Insights represent cross-conversation knowledge synthesized from Layer 2 Artifacts.
//! While an Artifact captures a specific piece of knowledge from a single Resource,
//! an Insight aggregates patterns, summaries, correlations, and trends across
//! multiple Artifacts with a confidence score indicating synthesis certainty.

use crate::config::SensitivityLevel;
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A cross-conversation knowledge insight synthesized from multiple Artifacts (Layer 3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    /// Unique insight identifier
    pub id: Uuid,
    /// IDs of the source Artifact(s) this insight was derived from
    pub source_artifact_ids: Vec<Uuid>,
    /// Type of insight
    pub insight_type: InsightType,
    /// The synthesized knowledge content
    pub content: String,
    /// Confidence score (0.0–1.0) indicating synthesis certainty
    pub confidence: f32,
    /// Sensitivity level (max of source artifacts)
    pub sensitivity: SensitivityLevel,
    /// Importance score (0.0–1.0)
    pub importance: f32,
    /// Number of evidence items supporting this insight
    pub evidence_count: u32,
    /// Searchable tags
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last time this insight was accessed
    pub last_accessed: Option<DateTime<Utc>>,
    /// Number of times this insight has been accessed
    pub access_count: u32,
    /// Arbitrary metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Insight {
    /// Record an access, incrementing the counter and updating the timestamp.
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Some(Utc::now());
    }

    /// Calculate relevance score based on importance and recency.
    ///
    /// Formula: `importance * 0.7 + recency_decay * 0.3`
    /// Recency uses a 30-day exponential decay (matching Artifact).
    pub fn relevance_score(&self) -> f32 {
        let now = Utc::now();
        let reference_time = self.last_accessed.unwrap_or(self.created_at);
        let age_seconds = (now - reference_time).num_seconds() as f32;
        let age_days = age_seconds / 86400.0;
        let decay = (-age_days / 30.0).exp();

        self.importance * 0.7 + decay * 0.3
    }
}

/// The type of cross-conversation insight
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightType {
    /// A recurring pattern detected across artifacts
    Pattern,
    /// An aggregated summary of related artifacts
    Summary,
    /// A correlation between co-occurring artifacts
    Correlation,
    /// A temporal trend observed over time
    Trend,
}

/// Builder for constructing `Insight` instances
pub struct InsightBuilder {
    source_artifact_ids: Vec<Uuid>,
    insight_type: InsightType,
    content: Option<String>,
    confidence: f32,
    sensitivity: SensitivityLevel,
    importance: f32,
    evidence_count: u32,
    tags: Vec<String>,
    metadata: HashMap<String, serde_json::Value>,
}

impl InsightBuilder {
    /// Create a new builder with the required insight type
    pub fn new(insight_type: InsightType) -> Self {
        Self {
            source_artifact_ids: Vec::new(),
            insight_type,
            content: None,
            confidence: 0.0,
            sensitivity: SensitivityLevel::Normal,
            importance: 0.0,
            evidence_count: 0,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a single source artifact ID
    pub fn source_artifact(mut self, id: Uuid) -> Self {
        self.source_artifact_ids.push(id);
        self
    }

    /// Add multiple source artifact IDs at once
    pub fn source_artifacts(mut self, ids: impl IntoIterator<Item = Uuid>) -> Self {
        self.source_artifact_ids.extend(ids);
        self
    }

    /// Set the synthesized knowledge content
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set the confidence score (clamped to 0.0–1.0)
    pub fn confidence(mut self, score: f32) -> Self {
        self.confidence = score.clamp(0.0, 1.0);
        self
    }

    /// Set the sensitivity level
    pub fn sensitivity(mut self, level: SensitivityLevel) -> Self {
        self.sensitivity = level;
        self
    }

    /// Set the importance score (clamped to 0.0–1.0)
    pub fn importance(mut self, score: f32) -> Self {
        self.importance = score.clamp(0.0, 1.0);
        self
    }

    /// Set the evidence count
    pub fn evidence_count(mut self, count: u32) -> Self {
        self.evidence_count = count;
        self
    }

    /// Add a tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add a metadata entry
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Build the insight, returning an error if content is missing
    pub fn build(self) -> Result<Insight> {
        let content = self
            .content
            .filter(|c| !c.is_empty())
            .ok_or_else(|| Error::Memory("insight content is required".to_string()))?;

        Ok(Insight {
            id: Uuid::new_v4(),
            source_artifact_ids: self.source_artifact_ids,
            insight_type: self.insight_type,
            content,
            confidence: self.confidence,
            sensitivity: self.sensitivity,
            importance: self.importance,
            evidence_count: self.evidence_count,
            tags: self.tags,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            metadata: self.metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insight_builder() {
        let artifact_id = Uuid::new_v4();
        let insight = InsightBuilder::new(InsightType::Pattern)
            .source_artifact(artifact_id)
            .content("test@example.com appears frequently")
            .confidence(0.8)
            .sensitivity(SensitivityLevel::Sensitive)
            .importance(0.7)
            .evidence_count(5)
            .tag("entity_frequency")
            .metadata("source", serde_json::Value::String("synthesizer".to_string()))
            .build()
            .unwrap();

        assert_eq!(insight.source_artifact_ids, vec![artifact_id]);
        assert_eq!(insight.insight_type, InsightType::Pattern);
        assert_eq!(insight.content, "test@example.com appears frequently");
        assert!((insight.confidence - 0.8).abs() < f32::EPSILON);
        assert_eq!(insight.sensitivity, SensitivityLevel::Sensitive);
        assert!((insight.importance - 0.7).abs() < f32::EPSILON);
        assert_eq!(insight.evidence_count, 5);
        assert_eq!(insight.tags, vec!["entity_frequency"]);
        assert_eq!(insight.access_count, 0);
        assert!(insight.last_accessed.is_none());
        assert_eq!(insight.metadata.len(), 1);
    }

    #[test]
    fn test_insight_builder_source_artifacts_plural() {
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        let insight = InsightBuilder::new(InsightType::Summary)
            .source_artifacts(ids.clone())
            .content("aggregated summary")
            .build()
            .unwrap();

        assert_eq!(insight.source_artifact_ids, ids);
    }

    #[test]
    fn test_insight_builder_defaults() {
        let insight = InsightBuilder::new(InsightType::Trend)
            .content("some trend")
            .build()
            .unwrap();

        assert!(insight.source_artifact_ids.is_empty());
        assert_eq!(insight.insight_type, InsightType::Trend);
        assert_eq!(insight.sensitivity, SensitivityLevel::Normal);
        assert!((insight.importance - 0.0).abs() < f32::EPSILON);
        assert!((insight.confidence - 0.0).abs() < f32::EPSILON);
        assert_eq!(insight.evidence_count, 0);
        assert!(insight.tags.is_empty());
        assert_eq!(insight.access_count, 0);
        assert!(insight.last_accessed.is_none());
    }

    #[test]
    fn test_insight_builder_missing_content() {
        let result = InsightBuilder::new(InsightType::Pattern).build();
        assert!(result.is_err());

        let result = InsightBuilder::new(InsightType::Pattern)
            .content("")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_insight_builder_clamps_confidence() {
        let high = InsightBuilder::new(InsightType::Pattern)
            .content("test")
            .confidence(1.5)
            .build()
            .unwrap();
        assert!((high.confidence - 1.0).abs() < f32::EPSILON);

        let low = InsightBuilder::new(InsightType::Pattern)
            .content("test")
            .confidence(-0.5)
            .build()
            .unwrap();
        assert!((low.confidence - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_insight_builder_clamps_importance() {
        let high = InsightBuilder::new(InsightType::Pattern)
            .content("test")
            .importance(2.0)
            .build()
            .unwrap();
        assert!((high.importance - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_insight_type_serialization() {
        let types = vec![
            InsightType::Pattern,
            InsightType::Summary,
            InsightType::Correlation,
            InsightType::Trend,
        ];

        for it in types {
            let json = serde_json::to_string(&it).unwrap();
            let deserialized: InsightType = serde_json::from_str(&json).unwrap();
            assert_eq!(it, deserialized);
        }
    }

    #[test]
    fn test_relevance_score_importance_dominant() {
        let insight = Insight {
            id: Uuid::new_v4(),
            source_artifact_ids: vec![],
            insight_type: InsightType::Pattern,
            content: "important pattern".to_string(),
            confidence: 0.9,
            sensitivity: SensitivityLevel::Sensitive,
            importance: 1.0,
            evidence_count: 5,
            tags: vec![],
            created_at: Utc::now() - chrono::Duration::days(90),
            last_accessed: None,
            access_count: 0,
            metadata: HashMap::new(),
        };

        let score = insight.relevance_score();
        assert!(score > 0.7, "score {score} should be > 0.7 from importance");
        assert!(score < 0.8, "score {score} should be < 0.8 (decay is small)");
    }

    #[test]
    fn test_relevance_score_fresh_insight() {
        let insight = Insight {
            id: Uuid::new_v4(),
            source_artifact_ids: vec![],
            insight_type: InsightType::Summary,
            content: "recent summary".to_string(),
            confidence: 0.7,
            sensitivity: SensitivityLevel::Normal,
            importance: 0.5,
            evidence_count: 2,
            tags: vec![],
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            metadata: HashMap::new(),
        };

        let score = insight.relevance_score();
        assert!(score > 0.6, "score {score} should be > 0.6 for fresh insight");
        assert!(score <= 1.0, "score {score} should be <= 1.0");
    }

    #[test]
    fn test_record_access() {
        let mut insight = InsightBuilder::new(InsightType::Pattern)
            .content("test")
            .build()
            .unwrap();

        assert_eq!(insight.access_count, 0);
        assert!(insight.last_accessed.is_none());

        insight.record_access();
        assert_eq!(insight.access_count, 1);
        assert!(insight.last_accessed.is_some());

        insight.record_access();
        assert_eq!(insight.access_count, 2);
    }

    #[test]
    fn test_insight_builder_clamps_importance_negative() {
        let insight = InsightBuilder::new(InsightType::Summary)
            .content("test")
            .importance(-1.0)
            .build()
            .unwrap();
        assert!((insight.importance - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_insight_builder_source_artifacts_combined() {
        let single_id = Uuid::new_v4();
        let batch_ids: Vec<Uuid> = (0..2).map(|_| Uuid::new_v4()).collect();
        let insight = InsightBuilder::new(InsightType::Correlation)
            .source_artifact(single_id)
            .source_artifacts(batch_ids.clone())
            .content("combined sources")
            .build()
            .unwrap();

        assert_eq!(insight.source_artifact_ids.len(), 3);
        assert_eq!(insight.source_artifact_ids[0], single_id);
        assert_eq!(insight.source_artifact_ids[1], batch_ids[0]);
        assert_eq!(insight.source_artifact_ids[2], batch_ids[1]);
    }

    #[test]
    fn test_relevance_score_uses_last_accessed() {
        // Two identical insights, but one was recently accessed
        let old = Insight {
            id: Uuid::new_v4(),
            source_artifact_ids: vec![],
            insight_type: InsightType::Pattern,
            content: "old access".to_string(),
            confidence: 0.5,
            sensitivity: SensitivityLevel::Normal,
            importance: 0.5,
            evidence_count: 1,
            tags: vec![],
            created_at: Utc::now() - chrono::Duration::days(60),
            last_accessed: Some(Utc::now() - chrono::Duration::days(60)),
            access_count: 1,
            metadata: HashMap::new(),
        };

        let mut fresh = old.clone();
        fresh.id = Uuid::new_v4();
        fresh.last_accessed = Some(Utc::now());

        // The freshly accessed insight should score higher due to recency
        assert!(
            fresh.relevance_score() > old.relevance_score(),
            "fresh ({}) should score higher than old ({})",
            fresh.relevance_score(),
            old.relevance_score()
        );
    }

    #[test]
    fn test_insight_serialization_round_trip() {
        let insight = InsightBuilder::new(InsightType::Correlation)
            .source_artifact(Uuid::new_v4())
            .content("round-trip test")
            .confidence(0.75)
            .sensitivity(SensitivityLevel::Sensitive)
            .importance(0.6)
            .evidence_count(3)
            .tag("test_tag")
            .metadata("key", serde_json::json!("value"))
            .build()
            .unwrap();

        let json = serde_json::to_string(&insight).unwrap();
        let deserialized: Insight = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, insight.id);
        assert_eq!(deserialized.insight_type, InsightType::Correlation);
        assert_eq!(deserialized.content, "round-trip test");
        assert!((deserialized.confidence - 0.75).abs() < f32::EPSILON);
        assert_eq!(deserialized.sensitivity, SensitivityLevel::Sensitive);
        assert!((deserialized.importance - 0.6).abs() < f32::EPSILON);
        assert_eq!(deserialized.evidence_count, 3);
        assert_eq!(deserialized.tags, vec!["test_tag"]);
        assert_eq!(deserialized.source_artifact_ids, insight.source_artifact_ids);
    }

    #[test]
    fn test_insight_builder_generates_unique_ids() {
        let a = InsightBuilder::new(InsightType::Pattern)
            .content("first")
            .build()
            .unwrap();
        let b = InsightBuilder::new(InsightType::Pattern)
            .content("second")
            .build()
            .unwrap();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn test_relevance_score_zero_importance_old() {
        // Zero importance, old insight → score should be near zero
        let insight = Insight {
            id: Uuid::new_v4(),
            source_artifact_ids: vec![],
            insight_type: InsightType::Trend,
            content: "zero importance old".to_string(),
            confidence: 0.5,
            sensitivity: SensitivityLevel::Normal,
            importance: 0.0,
            evidence_count: 0,
            tags: vec![],
            created_at: Utc::now() - chrono::Duration::days(365),
            last_accessed: None,
            access_count: 0,
            metadata: HashMap::new(),
        };

        let score = insight.relevance_score();
        assert!(score < 0.01, "score {score} should be near zero for old, unimportant insight");
        assert!(score >= 0.0, "score {score} should be non-negative");
    }
}
