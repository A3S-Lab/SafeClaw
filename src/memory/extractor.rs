//! Rule-based extraction of Layer 2 Artifacts from Layer 1 Resources
//!
//! The Extractor examines a classified Resource and produces zero or more
//! Artifacts. Extraction rules are deterministic (no LLM):
//!
//! 1. Each `Match` in the classification result produces an Entity artifact.
//! 2. The Resource's `ContentType` produces a Topic artifact.
//!
//! All extracted artifacts inherit the source Resource's sensitivity level.
//! Importance is derived from sensitivity.

use super::artifact::{Artifact, ArtifactBuilder, ArtifactType};
use super::resource::Resource;
use crate::config::SensitivityLevel;

/// Rule-based extractor that produces Artifacts from classified Resources.
pub struct Extractor;

impl Extractor {
    /// Extract structured knowledge artifacts from a classified Resource.
    ///
    /// Returns a (possibly empty) list of artifacts. Each classification match
    /// becomes an Entity artifact, and the content type becomes a Topic artifact.
    pub fn extract(resource: &Resource) -> Vec<Artifact> {
        let mut artifacts = Vec::new();
        let importance = Self::importance_from_sensitivity(resource.sensitivity);

        // Extract entities from classification matches
        if let Some(ref classification) = resource.classification {
            for m in &classification.matches {
                if let Ok(artifact) = ArtifactBuilder::new(ArtifactType::Entity)
                    .source_resource(resource.id)
                    .content(&m.redacted)
                    .sensitivity(resource.sensitivity)
                    .importance(importance)
                    .tag(&m.rule_name)
                    .build()
                {
                    artifacts.push(artifact);
                }
            }
        }

        // Extract topic from content type
        let topic_tag = Self::content_type_tag(&resource.content_type);
        if let Ok(artifact) = ArtifactBuilder::new(ArtifactType::Topic)
            .source_resource(resource.id)
            .content(format!("content type: {topic_tag}"))
            .sensitivity(resource.sensitivity)
            .importance(importance)
            .tag(topic_tag)
            .build()
        {
            artifacts.push(artifact);
        }

        artifacts
    }

    /// Map sensitivity level to an importance score.
    fn importance_from_sensitivity(level: SensitivityLevel) -> f32 {
        match level {
            SensitivityLevel::Public => 0.2,
            SensitivityLevel::Normal => 0.4,
            SensitivityLevel::Sensitive => 0.7,
            SensitivityLevel::HighlySensitive => 0.9,
        }
    }

    /// Map ContentType to a lowercase tag string.
    fn content_type_tag(ct: &super::resource::ContentType) -> &'static str {
        use super::resource::ContentType;
        match ct {
            ContentType::Text => "text",
            ContentType::Image => "image",
            ContentType::Audio => "audio",
            ContentType::Video => "video",
            ContentType::Document => "document",
            ContentType::Code => "code",
            ContentType::ToolOutput => "tool_output",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{default_classification_rules, SensitivityLevel};
    use crate::memory::artifact::ArtifactType;
    use crate::memory::resource::{ContentType, ResourceBuilder, StorageLocation};
    use crate::privacy::Classifier;

    /// Helper: build a Resource from text, running it through the real classifier
    fn classified_resource(text: &str, content_type: ContentType) -> Resource {
        let classifier =
            Classifier::new(default_classification_rules(), SensitivityLevel::Normal).unwrap();
        let classification = classifier.classify(text);
        let sensitivity = classification.level;

        ResourceBuilder::new(content_type)
            .user_id("user-1")
            .channel_id("telegram")
            .chat_id("chat-1")
            .raw_content(text.as_bytes().to_vec())
            .text_content(text)
            .sensitivity(sensitivity)
            .classification(classification)
            .storage_location(StorageLocation::Memory)
            .build()
            .unwrap()
    }

    #[test]
    fn test_extract_entities_from_email() {
        let resource = classified_resource("Contact me at test@example.com", ContentType::Text);
        let artifacts = Extractor::extract(&resource);

        // Should have 1 Entity (email) + 1 Topic (text)
        assert_eq!(artifacts.len(), 2);

        let entity = artifacts.iter().find(|a| a.artifact_type == ArtifactType::Entity).unwrap();
        assert_eq!(entity.tags, vec!["email"]);
        assert_eq!(entity.source_resource_ids, vec![resource.id]);
        assert_eq!(entity.sensitivity, SensitivityLevel::Sensitive);
    }

    #[test]
    fn test_extract_entities_from_credit_card() {
        let resource =
            classified_resource("Card: 4111-1111-1111-1111", ContentType::Text);
        let artifacts = Extractor::extract(&resource);

        let entity = artifacts.iter().find(|a| a.artifact_type == ArtifactType::Entity).unwrap();
        assert_eq!(entity.tags, vec!["credit_card"]);
        assert_eq!(entity.sensitivity, SensitivityLevel::HighlySensitive);
        assert!((entity.importance - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_extract_multiple_entities() {
        let resource = classified_resource(
            "SSN: 123-45-6789 and email test@example.com",
            ContentType::Text,
        );
        let artifacts = Extractor::extract(&resource);

        let entities: Vec<_> = artifacts
            .iter()
            .filter(|a| a.artifact_type == ArtifactType::Entity)
            .collect();
        assert_eq!(entities.len(), 2);
    }

    #[test]
    fn test_extract_no_matches() {
        let resource = classified_resource("Hello, how are you today?", ContentType::Text);
        let artifacts = Extractor::extract(&resource);

        // No classification matches → only the Topic artifact
        let entities: Vec<_> = artifacts
            .iter()
            .filter(|a| a.artifact_type == ArtifactType::Entity)
            .collect();
        assert!(entities.is_empty());

        let topics: Vec<_> = artifacts
            .iter()
            .filter(|a| a.artifact_type == ArtifactType::Topic)
            .collect();
        assert_eq!(topics.len(), 1);
    }

    #[test]
    fn test_extract_inherits_sensitivity() {
        let resource = classified_resource("test@example.com", ContentType::Text);
        let artifacts = Extractor::extract(&resource);

        for artifact in &artifacts {
            assert_eq!(
                artifact.sensitivity, resource.sensitivity,
                "artifact sensitivity should match resource sensitivity"
            );
        }
    }

    #[test]
    fn test_extract_topic_from_content_type() {
        let resource = classified_resource("fn main() {}", ContentType::Code);
        let artifacts = Extractor::extract(&resource);

        let topic = artifacts.iter().find(|a| a.artifact_type == ArtifactType::Topic).unwrap();
        assert_eq!(topic.tags, vec!["code"]);
        assert!(topic.content.contains("code"));
    }
}
