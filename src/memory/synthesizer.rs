//! Rule-based synthesis of Layer 3 Insights from Layer 2 Artifacts
//!
//! The Synthesizer examines a collection of Artifacts and produces zero or more
//! Insights. Synthesis rules are deterministic (no LLM):
//!
//! 1. **Entity frequency** — Entity artifacts with identical content appearing
//!    ≥2 times produce a Pattern insight with confidence proportional to count.
//! 2. **Topic aggregation** — Topic artifacts sharing a primary tag appearing
//!    ≥2 times produce a Summary insight.
//! 3. **Co-occurrence** — Entity artifacts sharing source resource IDs produce
//!    Correlation insights for each unique pair.
//!
//! All synthesized insights inherit the maximum sensitivity and average
//! importance of their source artifacts.

use super::artifact::{Artifact, ArtifactType};
use super::insight::{InsightBuilder, InsightType};
use crate::config::SensitivityLevel;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Rule-based synthesizer that produces Insights from Artifacts.
pub struct Synthesizer;

impl Synthesizer {
    /// Synthesize cross-conversation insights from a collection of Artifacts.
    ///
    /// Returns a (possibly empty) list of insights. Each synthesis rule is
    /// applied independently and the results are combined.
    pub fn synthesize(artifacts: &[Artifact]) -> Vec<super::insight::Insight> {
        let mut insights = Vec::new();

        insights.extend(Self::entity_frequency(artifacts));
        insights.extend(Self::topic_aggregation(artifacts));
        insights.extend(Self::co_occurrence(artifacts));

        insights
    }

    /// Rule 1: Group Entity artifacts by content. If ≥2 share the same content,
    /// produce a Pattern insight.
    fn entity_frequency(artifacts: &[Artifact]) -> Vec<super::insight::Insight> {
        let mut groups: HashMap<&str, Vec<&Artifact>> = HashMap::new();

        for artifact in artifacts {
            if artifact.artifact_type == ArtifactType::Entity {
                groups.entry(&artifact.content).or_default().push(artifact);
            }
        }

        let mut results = Vec::new();
        for (content, group) in &groups {
            if group.len() < 2 {
                continue;
            }

            let count = group.len() as f32;
            let confidence = (count / 5.0).min(1.0);
            let sensitivity = Self::max_sensitivity(group);
            let importance = Self::avg_importance(group);
            let ids: Vec<Uuid> = group.iter().map(|a| a.id).collect();

            // Union taint labels from all source artifacts
            let taint_union: HashSet<String> = group
                .iter()
                .flat_map(|a| a.taint_labels.iter().cloned())
                .collect();

            // Collect tags from all source artifacts, deduplicated
            let mut tags: Vec<String> = group
                .iter()
                .flat_map(|a| a.tags.iter().cloned())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            tags.push("entity_frequency".to_string());

            if let Ok(insight) = InsightBuilder::new(InsightType::Pattern)
                .source_artifacts(ids)
                .content(format!(
                    "{content} is a frequently referenced entity appearing {} times",
                    group.len()
                ))
                .confidence(confidence)
                .sensitivity(sensitivity)
                .importance(importance)
                .evidence_count(group.len() as u32)
                .tag("entity_frequency")
                .taint_labels(taint_union)
                .build()
            {
                // Replace the single tag with the full deduplicated set
                let mut insight = insight;
                insight.tags = tags;
                results.push(insight);
            }
        }
        results
    }

    /// Rule 2: Group Topic artifacts by primary tag. If ≥2 share a tag,
    /// produce a Summary insight.
    fn topic_aggregation(artifacts: &[Artifact]) -> Vec<super::insight::Insight> {
        let mut groups: HashMap<&str, Vec<&Artifact>> = HashMap::new();

        for artifact in artifacts {
            if artifact.artifact_type == ArtifactType::Topic {
                if let Some(primary_tag) = artifact.tags.first() {
                    groups.entry(primary_tag).or_default().push(artifact);
                }
            }
        }

        let mut results = Vec::new();
        for (tag, group) in &groups {
            if group.len() < 2 {
                continue;
            }

            let count = group.len() as f32;
            let confidence = (count / 5.0).min(1.0);
            let sensitivity = Self::max_sensitivity(group);
            let importance = Self::avg_importance(group);
            let ids: Vec<Uuid> = group.iter().map(|a| a.id).collect();

            // Union taint labels from all source artifacts
            let taint_union: HashSet<String> = group
                .iter()
                .flat_map(|a| a.taint_labels.iter().cloned())
                .collect();

            if let Ok(insight) = InsightBuilder::new(InsightType::Summary)
                .source_artifacts(ids)
                .content(format!(
                    "topic '{tag}' appears across {} artifacts",
                    group.len()
                ))
                .confidence(confidence)
                .sensitivity(sensitivity)
                .importance(importance)
                .evidence_count(group.len() as u32)
                .tag(tag.to_string())
                .tag("topic_aggregation")
                .taint_labels(taint_union)
                .build()
            {
                results.push(insight);
            }
        }
        results
    }

    /// Rule 3: Entity artifacts sharing source_resource_ids produce Correlation
    /// insights for each unique pair.
    fn co_occurrence(artifacts: &[Artifact]) -> Vec<super::insight::Insight> {
        // Build a map: resource_id → [entity artifacts referencing it]
        let mut resource_to_entities: HashMap<Uuid, Vec<&Artifact>> = HashMap::new();

        for artifact in artifacts {
            if artifact.artifact_type == ArtifactType::Entity {
                for resource_id in &artifact.source_resource_ids {
                    resource_to_entities
                        .entry(*resource_id)
                        .or_default()
                        .push(artifact);
                }
            }
        }

        // For each resource with ≥2 entities, create pairs
        let mut seen_pairs: HashSet<(Uuid, Uuid)> = HashSet::new();
        let mut results = Vec::new();

        for entities in resource_to_entities.values() {
            if entities.len() < 2 {
                continue;
            }

            for i in 0..entities.len() {
                for j in (i + 1)..entities.len() {
                    let a = entities[i];
                    let b = entities[j];

                    // Normalize pair ordering to avoid duplicates
                    let pair = if a.id < b.id {
                        (a.id, b.id)
                    } else {
                        (b.id, a.id)
                    };

                    if !seen_pairs.insert(pair) {
                        continue;
                    }

                    let sensitivity = Self::max_sensitivity(&[a, b]);
                    let importance = Self::avg_importance(&[a, b]);

                    // Union taint labels from both artifacts
                    let taint_union: HashSet<String> = a
                        .taint_labels
                        .iter()
                        .chain(b.taint_labels.iter())
                        .cloned()
                        .collect();

                    if let Ok(insight) = InsightBuilder::new(InsightType::Correlation)
                        .source_artifact(a.id)
                        .source_artifact(b.id)
                        .content(format!(
                            "'{}' and '{}' co-occur in the same resource",
                            a.content, b.content
                        ))
                        .confidence(0.6)
                        .sensitivity(sensitivity)
                        .importance(importance)
                        .evidence_count(2)
                        .tag("co_occurrence")
                        .taint_labels(taint_union)
                        .build()
                    {
                        results.push(insight);
                    }
                }
            }
        }
        results
    }

    /// Return the maximum sensitivity level from a slice of artifacts.
    fn max_sensitivity(artifacts: &[&Artifact]) -> SensitivityLevel {
        artifacts
            .iter()
            .map(|a| a.sensitivity)
            .max()
            .unwrap_or(SensitivityLevel::Normal)
    }

    /// Return the average importance score from a slice of artifacts.
    fn avg_importance(artifacts: &[&Artifact]) -> f32 {
        if artifacts.is_empty() {
            return 0.0;
        }
        let sum: f32 = artifacts.iter().map(|a| a.importance).sum();
        sum / artifacts.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SensitivityLevel;
    use crate::memory::artifact::ArtifactBuilder;

    /// Helper: build an Entity artifact with given content, tags, and resource IDs
    fn entity(
        content: &str,
        sensitivity: SensitivityLevel,
        importance: f32,
        tags: &[&str],
        resource_ids: &[Uuid],
    ) -> Artifact {
        let mut builder = ArtifactBuilder::new(ArtifactType::Entity)
            .content(content)
            .sensitivity(sensitivity)
            .importance(importance);
        for tag in tags {
            builder = builder.tag(*tag);
        }
        for id in resource_ids {
            builder = builder.source_resource(*id);
        }
        builder.build().unwrap()
    }

    /// Helper: build a Topic artifact with a primary tag
    fn topic(tag: &str, sensitivity: SensitivityLevel, importance: f32) -> Artifact {
        ArtifactBuilder::new(ArtifactType::Topic)
            .content(format!("content type: {tag}"))
            .sensitivity(sensitivity)
            .importance(importance)
            .tag(tag)
            .build()
            .unwrap()
    }

    #[test]
    fn test_entity_frequency_pattern() {
        let a1 = entity(
            "test@example.com",
            SensitivityLevel::Sensitive,
            0.7,
            &["email"],
            &[],
        );
        let a2 = entity(
            "test@example.com",
            SensitivityLevel::Sensitive,
            0.7,
            &["email"],
            &[],
        );

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let patterns: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Pattern)
            .collect();
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].content.contains("test@example.com"));
        assert!(patterns[0].content.contains("2 times"));
        assert!(patterns[0].tags.contains(&"entity_frequency".to_string()));
        assert_eq!(patterns[0].evidence_count, 2);
    }

    #[test]
    fn test_entity_frequency_below_threshold() {
        let a1 = entity(
            "single@example.com",
            SensitivityLevel::Normal,
            0.5,
            &["email"],
            &[],
        );

        let insights = Synthesizer::synthesize(&[a1]);

        let patterns: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Pattern)
            .collect();
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_entity_frequency_confidence_cap() {
        // 6 occurrences → confidence = min(6/5, 1.0) = 1.0
        let artifacts: Vec<Artifact> = (0..6)
            .map(|_| {
                entity(
                    "repeated@example.com",
                    SensitivityLevel::Normal,
                    0.5,
                    &[],
                    &[],
                )
            })
            .collect();

        let insights = Synthesizer::synthesize(&artifacts);

        let pattern = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Pattern)
            .unwrap();
        assert!((pattern.confidence - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_entity_frequency_distinct_entities() {
        let a1 = entity(
            "alice@example.com",
            SensitivityLevel::Normal,
            0.5,
            &["email"],
            &[],
        );
        let a2 = entity(
            "alice@example.com",
            SensitivityLevel::Normal,
            0.5,
            &["email"],
            &[],
        );
        let b1 = entity(
            "bob@example.com",
            SensitivityLevel::Normal,
            0.5,
            &["email"],
            &[],
        );
        let b2 = entity(
            "bob@example.com",
            SensitivityLevel::Normal,
            0.5,
            &["email"],
            &[],
        );

        let insights = Synthesizer::synthesize(&[a1, a2, b1, b2]);

        let patterns: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Pattern)
            .collect();
        assert_eq!(patterns.len(), 2);
    }

    #[test]
    fn test_topic_aggregation() {
        let t1 = topic("code", SensitivityLevel::Normal, 0.4);
        let t2 = topic("code", SensitivityLevel::Normal, 0.6);

        let insights = Synthesizer::synthesize(&[t1, t2]);

        let summaries: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Summary)
            .collect();
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].content.contains("code"));
        assert!(summaries[0].tags.contains(&"topic_aggregation".to_string()));
        assert!(summaries[0].tags.contains(&"code".to_string()));
    }

    #[test]
    fn test_topic_aggregation_different_tags() {
        let t1 = topic("code", SensitivityLevel::Normal, 0.4);
        let t2 = topic("text", SensitivityLevel::Normal, 0.4);

        let insights = Synthesizer::synthesize(&[t1, t2]);

        let summaries: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Summary)
            .collect();
        assert!(summaries.is_empty());
    }

    #[test]
    fn test_co_occurrence() {
        let resource_id = Uuid::new_v4();
        let a1 = entity(
            "alice@example.com",
            SensitivityLevel::Normal,
            0.5,
            &["email"],
            &[resource_id],
        );
        let a2 = entity(
            "555-1234",
            SensitivityLevel::Sensitive,
            0.7,
            &["phone"],
            &[resource_id],
        );

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let correlations: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Correlation)
            .collect();
        assert_eq!(correlations.len(), 1);
        assert!(correlations[0].tags.contains(&"co_occurrence".to_string()));
        assert!((correlations[0].confidence - 0.6).abs() < f32::EPSILON);
        assert_eq!(correlations[0].source_artifact_ids.len(), 2);
    }

    #[test]
    fn test_co_occurrence_no_shared_resource() {
        let r1 = Uuid::new_v4();
        let r2 = Uuid::new_v4();
        let a1 = entity(
            "alice@example.com",
            SensitivityLevel::Normal,
            0.5,
            &[],
            &[r1],
        );
        let a2 = entity("555-1234", SensitivityLevel::Normal, 0.5, &[], &[r2]);

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let correlations: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Correlation)
            .collect();
        assert!(correlations.is_empty());
    }

    #[test]
    fn test_co_occurrence_no_duplicate_pairs() {
        let resource_id = Uuid::new_v4();
        let a1 = entity(
            "alice@example.com",
            SensitivityLevel::Normal,
            0.5,
            &[],
            &[resource_id],
        );
        let a2 = entity(
            "bob@example.com",
            SensitivityLevel::Normal,
            0.5,
            &[],
            &[resource_id],
        );
        let a3 = entity(
            "555-1234",
            SensitivityLevel::Normal,
            0.5,
            &[],
            &[resource_id],
        );

        let insights = Synthesizer::synthesize(&[a1, a2, a3]);

        let correlations: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Correlation)
            .collect();
        // 3 entities in same resource → C(3,2) = 3 unique pairs
        assert_eq!(correlations.len(), 3);
    }

    #[test]
    fn test_sensitivity_max_propagation() {
        let a1 = entity("test@example.com", SensitivityLevel::Normal, 0.5, &[], &[]);
        let a2 = entity(
            "test@example.com",
            SensitivityLevel::HighlySensitive,
            0.5,
            &[],
            &[],
        );

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let pattern = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Pattern)
            .unwrap();
        assert_eq!(pattern.sensitivity, SensitivityLevel::HighlySensitive);
    }

    #[test]
    fn test_importance_average() {
        let a1 = entity("test@example.com", SensitivityLevel::Normal, 0.4, &[], &[]);
        let a2 = entity("test@example.com", SensitivityLevel::Normal, 0.8, &[], &[]);

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let pattern = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Pattern)
            .unwrap();
        assert!((pattern.importance - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn test_synthesize_empty_input() {
        let insights = Synthesizer::synthesize(&[]);
        assert!(insights.is_empty());
    }

    #[test]
    fn test_synthesize_no_qualifying_artifacts() {
        // Fact and Preference types are not processed by any synthesis rule
        let fact = ArtifactBuilder::new(ArtifactType::Fact)
            .content("the sky is blue")
            .build()
            .unwrap();
        let pref = ArtifactBuilder::new(ArtifactType::Preference)
            .content("prefers dark mode")
            .build()
            .unwrap();

        let insights = Synthesizer::synthesize(&[fact, pref]);
        assert!(insights.is_empty());
    }

    #[test]
    fn test_synthesize_combined_rules() {
        let resource_id = Uuid::new_v4();

        // 2× same entity → triggers entity_frequency (Pattern)
        let e1 = entity(
            "test@example.com",
            SensitivityLevel::Sensitive,
            0.7,
            &["email"],
            &[resource_id],
        );
        let e2 = entity(
            "test@example.com",
            SensitivityLevel::Sensitive,
            0.7,
            &["email"],
            &[resource_id],
        );
        // 2nd entity in same resource → triggers co_occurrence (Correlation)
        let e3 = entity(
            "555-1234",
            SensitivityLevel::Normal,
            0.5,
            &["phone"],
            &[resource_id],
        );
        // 2× same topic tag → triggers topic_aggregation (Summary)
        let t1 = topic("code", SensitivityLevel::Normal, 0.4);
        let t2 = topic("code", SensitivityLevel::Normal, 0.4);

        let insights = Synthesizer::synthesize(&[e1, e2, e3, t1, t2]);

        let patterns: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Pattern)
            .collect();
        let summaries: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Summary)
            .collect();
        let correlations: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Correlation)
            .collect();

        assert!(
            !patterns.is_empty(),
            "should have Pattern from entity_frequency"
        );
        assert!(
            !summaries.is_empty(),
            "should have Summary from topic_aggregation"
        );
        assert!(
            !correlations.is_empty(),
            "should have Correlation from co_occurrence"
        );
    }

    #[test]
    fn test_co_occurrence_dedup_across_resources() {
        // Same pair of entities in two different resources should still produce only 1 correlation
        let r1 = Uuid::new_v4();
        let r2 = Uuid::new_v4();
        let a1 = entity(
            "alice@example.com",
            SensitivityLevel::Normal,
            0.5,
            &[],
            &[r1, r2],
        );
        let a2 = entity(
            "bob@example.com",
            SensitivityLevel::Normal,
            0.5,
            &[],
            &[r1, r2],
        );

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let correlations: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Correlation)
            .collect();
        assert_eq!(
            correlations.len(),
            1,
            "same pair across 2 resources should produce 1 correlation"
        );
    }

    #[test]
    fn test_entity_frequency_tag_merging() {
        // Two entity artifacts with different tags → merged tags in the Pattern insight
        let a1 = entity(
            "test@example.com",
            SensitivityLevel::Normal,
            0.5,
            &["email"],
            &[],
        );
        let a2 = entity(
            "test@example.com",
            SensitivityLevel::Normal,
            0.5,
            &["contact"],
            &[],
        );

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let pattern = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Pattern)
            .unwrap();
        assert!(pattern.tags.contains(&"email".to_string()));
        assert!(pattern.tags.contains(&"contact".to_string()));
        assert!(pattern.tags.contains(&"entity_frequency".to_string()));
    }

    #[test]
    fn test_topic_aggregation_confidence_value() {
        // 3 topics with same tag → confidence = min(3/5, 1.0) = 0.6
        let t1 = topic("code", SensitivityLevel::Normal, 0.5);
        let t2 = topic("code", SensitivityLevel::Normal, 0.5);
        let t3 = topic("code", SensitivityLevel::Normal, 0.5);

        let insights = Synthesizer::synthesize(&[t1, t2, t3]);

        let summary = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Summary)
            .unwrap();
        assert!((summary.confidence - 0.6).abs() < f32::EPSILON);
        assert_eq!(summary.evidence_count, 3);
    }

    #[test]
    fn test_entity_frequency_confidence_value() {
        // 3 entities → confidence = min(3/5, 1.0) = 0.6
        let artifacts: Vec<Artifact> = (0..3)
            .map(|_| entity("same@example.com", SensitivityLevel::Normal, 0.5, &[], &[]))
            .collect();

        let insights = Synthesizer::synthesize(&artifacts);

        let pattern = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Pattern)
            .unwrap();
        assert!((pattern.confidence - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn test_co_occurrence_sensitivity_propagation() {
        let resource_id = Uuid::new_v4();
        let a1 = entity(
            "alice@example.com",
            SensitivityLevel::Public,
            0.3,
            &[],
            &[resource_id],
        );
        let a2 = entity(
            "555-1234",
            SensitivityLevel::HighlySensitive,
            0.9,
            &[],
            &[resource_id],
        );

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let correlation = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Correlation)
            .unwrap();
        assert_eq!(correlation.sensitivity, SensitivityLevel::HighlySensitive);
    }

    #[test]
    fn test_co_occurrence_importance_averaging() {
        let resource_id = Uuid::new_v4();
        let a1 = entity(
            "alice@example.com",
            SensitivityLevel::Normal,
            0.2,
            &[],
            &[resource_id],
        );
        let a2 = entity(
            "555-1234",
            SensitivityLevel::Normal,
            0.8,
            &[],
            &[resource_id],
        );

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let correlation = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Correlation)
            .unwrap();
        assert!((correlation.importance - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_topic_without_tags_not_grouped() {
        // Topic artifacts with no tags should not trigger aggregation
        let t1 = ArtifactBuilder::new(ArtifactType::Topic)
            .content("untagged topic 1")
            .build()
            .unwrap();
        let t2 = ArtifactBuilder::new(ArtifactType::Topic)
            .content("untagged topic 2")
            .build()
            .unwrap();

        let insights = Synthesizer::synthesize(&[t1, t2]);

        let summaries: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Summary)
            .collect();
        assert!(summaries.is_empty());
    }

    #[test]
    fn test_entity_no_resource_ids_no_co_occurrence() {
        // Entity artifacts without source_resource_ids should not trigger co-occurrence
        let a1 = entity("alice@example.com", SensitivityLevel::Normal, 0.5, &[], &[]);
        let a2 = entity("bob@example.com", SensitivityLevel::Normal, 0.5, &[], &[]);

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let correlations: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Correlation)
            .collect();
        assert!(correlations.is_empty());
    }

    #[test]
    fn test_synthesize_procedure_artifacts_ignored() {
        let p1 = ArtifactBuilder::new(ArtifactType::Procedure)
            .content("step 1: do X")
            .tag("setup")
            .build()
            .unwrap();
        let p2 = ArtifactBuilder::new(ArtifactType::Procedure)
            .content("step 2: do Y")
            .tag("setup")
            .build()
            .unwrap();

        let insights = Synthesizer::synthesize(&[p1, p2]);
        assert!(
            insights.is_empty(),
            "Procedure artifacts should not trigger any synthesis rule"
        );
    }

    #[test]
    fn test_topic_aggregation_sensitivity_propagation() {
        let t1 = topic("code", SensitivityLevel::Public, 0.3);
        let t2 = topic("code", SensitivityLevel::Sensitive, 0.7);

        let insights = Synthesizer::synthesize(&[t1, t2]);

        let summary = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Summary)
            .unwrap();
        assert_eq!(summary.sensitivity, SensitivityLevel::Sensitive);
    }

    #[test]
    fn test_topic_aggregation_importance_averaging() {
        let t1 = topic("code", SensitivityLevel::Normal, 0.2);
        let t2 = topic("code", SensitivityLevel::Normal, 0.8);

        let insights = Synthesizer::synthesize(&[t1, t2]);

        let summary = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Summary)
            .unwrap();
        assert!((summary.importance - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_entity_frequency_source_artifact_ids_tracked() {
        let a1 = entity("test@example.com", SensitivityLevel::Normal, 0.5, &[], &[]);
        let a2 = entity("test@example.com", SensitivityLevel::Normal, 0.5, &[], &[]);
        let id1 = a1.id;
        let id2 = a2.id;

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let pattern = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Pattern)
            .unwrap();
        assert_eq!(pattern.source_artifact_ids.len(), 2);
        assert!(pattern.source_artifact_ids.contains(&id1));
        assert!(pattern.source_artifact_ids.contains(&id2));
    }

    #[test]
    fn test_co_occurrence_content_includes_both_entities() {
        let resource_id = Uuid::new_v4();
        let a1 = entity(
            "alice@example.com",
            SensitivityLevel::Normal,
            0.5,
            &[],
            &[resource_id],
        );
        let a2 = entity(
            "555-1234",
            SensitivityLevel::Normal,
            0.5,
            &[],
            &[resource_id],
        );

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let correlation = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Correlation)
            .unwrap();
        assert!(correlation.content.contains("alice@example.com"));
        assert!(correlation.content.contains("555-1234"));
        assert!(correlation.content.contains("co-occur"));
    }

    #[test]
    fn test_entity_frequency_taint_propagation() {
        let mut a1 = entity("test@example.com", SensitivityLevel::Sensitive, 0.7, &[], &[]);
        a1.taint_labels.insert("pii:email".to_string());
        a1.taint_labels.insert("session:abc".to_string());

        let mut a2 = entity("test@example.com", SensitivityLevel::Sensitive, 0.7, &[], &[]);
        a2.taint_labels.insert("pii:email".to_string());
        a2.taint_labels.insert("session:def".to_string());

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let pattern = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Pattern)
            .unwrap();
        // Union of all taint labels
        assert_eq!(pattern.taint_labels.len(), 3);
        assert!(pattern.taint_labels.contains("pii:email"));
        assert!(pattern.taint_labels.contains("session:abc"));
        assert!(pattern.taint_labels.contains("session:def"));
    }

    #[test]
    fn test_topic_aggregation_taint_propagation() {
        let mut t1 = topic("code", SensitivityLevel::Normal, 0.4);
        t1.taint_labels.insert("src:channel-a".to_string());

        let mut t2 = topic("code", SensitivityLevel::Normal, 0.6);
        t2.taint_labels.insert("src:channel-b".to_string());

        let insights = Synthesizer::synthesize(&[t1, t2]);

        let summary = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Summary)
            .unwrap();
        assert_eq!(summary.taint_labels.len(), 2);
        assert!(summary.taint_labels.contains("src:channel-a"));
        assert!(summary.taint_labels.contains("src:channel-b"));
    }

    #[test]
    fn test_co_occurrence_taint_propagation() {
        let resource_id = Uuid::new_v4();
        let mut a1 = entity(
            "alice@example.com",
            SensitivityLevel::Normal,
            0.5,
            &[],
            &[resource_id],
        );
        a1.taint_labels.insert("pii:email".to_string());

        let mut a2 = entity(
            "555-1234",
            SensitivityLevel::Sensitive,
            0.7,
            &[],
            &[resource_id],
        );
        a2.taint_labels.insert("pii:phone".to_string());

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let correlation = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Correlation)
            .unwrap();
        assert_eq!(correlation.taint_labels.len(), 2);
        assert!(correlation.taint_labels.contains("pii:email"));
        assert!(correlation.taint_labels.contains("pii:phone"));
    }

    #[test]
    fn test_empty_taint_labels_propagated() {
        let a1 = entity("test@example.com", SensitivityLevel::Normal, 0.5, &[], &[]);
        let a2 = entity("test@example.com", SensitivityLevel::Normal, 0.5, &[], &[]);

        let insights = Synthesizer::synthesize(&[a1, a2]);

        let pattern = insights
            .iter()
            .find(|i| i.insight_type == InsightType::Pattern)
            .unwrap();
        assert!(pattern.taint_labels.is_empty());
    }
}
