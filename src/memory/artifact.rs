//! Layer 2 artifact data types
//!
//! Artifacts represent structured knowledge extracted from Layer 1 Resources.
//! While a Resource holds raw classified content, an Artifact captures a
//! specific piece of knowledge (entity, fact, topic, preference, procedure)
//! with importance scoring and access tracking for relevance queries.

use crate::config::SensitivityLevel;
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// A structured knowledge artifact extracted from one or more Resources (Layer 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Unique artifact identifier
    pub id: Uuid,
    /// IDs of the source Resource(s) this artifact was derived from
    pub source_resource_ids: Vec<Uuid>,
    /// Type of knowledge this artifact represents
    pub artifact_type: ArtifactType,
    /// The extracted knowledge content
    pub content: String,
    /// Inherited sensitivity level from source Resources
    pub sensitivity: SensitivityLevel,
    /// Importance score (0.0–1.0)
    pub importance: f32,
    /// Searchable tags
    pub tags: Vec<String>,
    /// Taint labels inherited from source Resources (union of all sources).
    /// Propagated to Insights derived from this Artifact.
    pub taint_labels: HashSet<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last time this artifact was accessed
    pub last_accessed: Option<DateTime<Utc>>,
    /// Number of times this artifact has been accessed
    pub access_count: u32,
    /// Arbitrary metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Artifact {
    /// Record an access, incrementing the counter and updating the timestamp.
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Some(Utc::now());
    }

    /// Calculate relevance score based on importance and recency.
    ///
    /// Formula: `importance * 0.7 + recency_decay * 0.3`
    /// Recency uses a 30-day exponential decay (matching a3s-code's MemoryItem).
    pub fn relevance_score(&self) -> f32 {
        let now = Utc::now();
        let reference_time = self.last_accessed.unwrap_or(self.created_at);
        let age_seconds = (now - reference_time).num_seconds() as f32;
        let age_days = age_seconds / 86400.0;
        let decay = (-age_days / 30.0).exp(); // 30-day half-life

        self.importance * 0.7 + decay * 0.3
    }
}

/// The type of knowledge an artifact represents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    /// A named entity (email, phone, person, etc.)
    Entity,
    /// A factual statement
    Fact,
    /// A topic or theme
    Topic,
    /// A user preference
    Preference,
    /// A procedural instruction
    Procedure,
}

/// Builder for constructing `Artifact` instances
pub struct ArtifactBuilder {
    source_resource_ids: Vec<Uuid>,
    artifact_type: ArtifactType,
    content: Option<String>,
    sensitivity: SensitivityLevel,
    importance: f32,
    tags: Vec<String>,
    taint_labels: HashSet<String>,
    metadata: HashMap<String, serde_json::Value>,
}

impl ArtifactBuilder {
    /// Create a new builder with the required artifact type
    pub fn new(artifact_type: ArtifactType) -> Self {
        Self {
            source_resource_ids: Vec::new(),
            artifact_type,
            content: None,
            sensitivity: SensitivityLevel::Normal,
            importance: 0.0,
            tags: Vec::new(),
            taint_labels: HashSet::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a source resource ID
    pub fn source_resource(mut self, id: Uuid) -> Self {
        self.source_resource_ids.push(id);
        self
    }

    /// Set the extracted knowledge content
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
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

    /// Add a tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add a taint label
    pub fn taint_label(mut self, label: impl Into<String>) -> Self {
        self.taint_labels.insert(label.into());
        self
    }

    /// Set taint labels from an iterator
    pub fn taint_labels(mut self, labels: impl IntoIterator<Item = String>) -> Self {
        self.taint_labels.extend(labels);
        self
    }

    /// Add a metadata entry
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Build the artifact, returning an error if content is missing
    pub fn build(self) -> Result<Artifact> {
        let content = self
            .content
            .filter(|c| !c.is_empty())
            .ok_or_else(|| Error::Memory("artifact content is required".to_string()))?;

        Ok(Artifact {
            id: Uuid::new_v4(),
            source_resource_ids: self.source_resource_ids,
            artifact_type: self.artifact_type,
            content,
            sensitivity: self.sensitivity,
            importance: self.importance,
            tags: self.tags,
            taint_labels: self.taint_labels,
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
    fn test_artifact_builder() {
        let resource_id = Uuid::new_v4();
        let artifact = ArtifactBuilder::new(ArtifactType::Entity)
            .source_resource(resource_id)
            .content("test@example.com")
            .sensitivity(SensitivityLevel::Sensitive)
            .importance(0.7)
            .tag("email")
            .metadata(
                "source",
                serde_json::Value::String("classifier".to_string()),
            )
            .build()
            .unwrap();

        assert_eq!(artifact.source_resource_ids, vec![resource_id]);
        assert_eq!(artifact.artifact_type, ArtifactType::Entity);
        assert_eq!(artifact.content, "test@example.com");
        assert_eq!(artifact.sensitivity, SensitivityLevel::Sensitive);
        assert!((artifact.importance - 0.7).abs() < f32::EPSILON);
        assert_eq!(artifact.tags, vec!["email"]);
        assert_eq!(artifact.access_count, 0);
        assert!(artifact.last_accessed.is_none());
        assert_eq!(artifact.metadata.len(), 1);
    }

    #[test]
    fn test_artifact_builder_defaults() {
        let artifact = ArtifactBuilder::new(ArtifactType::Fact)
            .content("the sky is blue")
            .build()
            .unwrap();

        assert!(artifact.source_resource_ids.is_empty());
        assert_eq!(artifact.artifact_type, ArtifactType::Fact);
        assert_eq!(artifact.sensitivity, SensitivityLevel::Normal);
        assert!((artifact.importance - 0.0).abs() < f32::EPSILON);
        assert!(artifact.tags.is_empty());
        assert_eq!(artifact.access_count, 0);
        assert!(artifact.last_accessed.is_none());
    }

    #[test]
    fn test_artifact_builder_missing_content() {
        let result = ArtifactBuilder::new(ArtifactType::Entity).build();
        assert!(result.is_err());

        // Empty string should also fail
        let result = ArtifactBuilder::new(ArtifactType::Entity)
            .content("")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_artifact_type_serialization() {
        let types = vec![
            ArtifactType::Entity,
            ArtifactType::Fact,
            ArtifactType::Topic,
            ArtifactType::Preference,
            ArtifactType::Procedure,
        ];

        for at in types {
            let json = serde_json::to_string(&at).unwrap();
            let deserialized: ArtifactType = serde_json::from_str(&json).unwrap();
            assert_eq!(at, deserialized);
        }
    }

    #[test]
    fn test_relevance_score_importance_dominant() {
        // High importance, created long ago — importance term should dominate
        let artifact = Artifact {
            id: Uuid::new_v4(),
            source_resource_ids: vec![],
            artifact_type: ArtifactType::Entity,
            content: "important entity".to_string(),
            sensitivity: SensitivityLevel::Sensitive,
            importance: 1.0,
            tags: vec![],
            taint_labels: HashSet::new(),
            created_at: Utc::now() - chrono::Duration::days(90),
            last_accessed: None,
            access_count: 0,
            metadata: HashMap::new(),
        };

        let score = artifact.relevance_score();
        // importance contributes 1.0 * 0.7 = 0.7
        // decay after 90 days ≈ exp(-3) ≈ 0.05, contributes ~0.015
        assert!(score > 0.7, "score {score} should be > 0.7 from importance");
        assert!(
            score < 0.8,
            "score {score} should be < 0.8 (decay is small)"
        );
    }

    #[test]
    fn test_relevance_score_fresh_artifact() {
        let artifact = Artifact {
            id: Uuid::new_v4(),
            source_resource_ids: vec![],
            artifact_type: ArtifactType::Fact,
            content: "recent fact".to_string(),
            sensitivity: SensitivityLevel::Normal,
            importance: 0.5,
            tags: vec![],
            taint_labels: HashSet::new(),
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            metadata: HashMap::new(),
        };

        let score = artifact.relevance_score();
        // importance contributes 0.5 * 0.7 = 0.35
        // decay ≈ 1.0 (just created), contributes ~0.3
        // total ≈ 0.65
        assert!(
            score > 0.6,
            "score {score} should be > 0.6 for fresh artifact"
        );
        assert!(score <= 1.0, "score {score} should be <= 1.0");
    }
}
