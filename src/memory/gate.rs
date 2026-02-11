//! Privacy Gate — wires Classifier + PolicyEngine to Resource creation
//!
//! Every inbound message passes through the gate, which classifies the
//! content, evaluates the routing policy, and produces a `Resource` with
//! the correct sensitivity and storage location.

use crate::error::Result;
use crate::privacy::{Classifier, PolicyDecision, PolicyEngine};

use super::resource::{ContentType, Resource, ResourceBuilder, StorageLocation};

/// Input to the Privacy Gate
pub struct GateInput {
    /// User who submitted this content
    pub user_id: String,
    /// Channel the content arrived on
    pub channel_id: String,
    /// Chat within the channel
    pub chat_id: String,
    /// Text content to classify (may be empty for binary-only)
    pub content: String,
    /// Content type
    pub content_type: ContentType,
    /// Raw binary content
    pub raw_content: Vec<u8>,
    /// Optional metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Privacy Gate that classifies input and routes it to the correct storage
pub struct PrivacyGate {
    classifier: Classifier,
    policy_engine: PolicyEngine,
}

impl PrivacyGate {
    /// Create a new gate with the given classifier and policy engine
    pub fn new(classifier: Classifier, policy_engine: PolicyEngine) -> Self {
        Self {
            classifier,
            policy_engine,
        }
    }

    /// Process an input through classification and policy evaluation,
    /// producing a routed `Resource`.
    pub fn process(&self, input: GateInput) -> Result<Resource> {
        // Classify the text content
        let classification = self.classifier.classify(&input.content);

        // Evaluate policy to determine routing
        let decision = self
            .policy_engine
            .evaluate(classification.level, None, None);

        // Map policy decision to storage location
        let storage_location = match decision {
            PolicyDecision::ProcessLocal => StorageLocation::Memory,
            PolicyDecision::ProcessInTee => StorageLocation::Tee {
                tee_ref: format!("tee-{}", uuid::Uuid::new_v4()),
            },
            PolicyDecision::Reject => StorageLocation::Memory,
            PolicyDecision::RequireConfirmation => StorageLocation::Memory,
        };

        let mut builder = ResourceBuilder::new(input.content_type)
            .user_id(input.user_id)
            .channel_id(input.channel_id)
            .chat_id(input.chat_id)
            .raw_content(input.raw_content)
            .sensitivity(classification.level)
            .classification(classification)
            .storage_location(storage_location);

        if !input.content.is_empty() {
            builder = builder.text_content(input.content);
        }

        for (key, value) in input.metadata {
            builder = builder.metadata(key, value);
        }

        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{default_classification_rules, SensitivityLevel};
    use crate::privacy::PolicyBuilder;
    use std::collections::HashMap;

    fn default_gate() -> PrivacyGate {
        let classifier =
            Classifier::new(default_classification_rules(), SensitivityLevel::Normal).unwrap();
        let policy_engine = PolicyEngine::new();
        PrivacyGate::new(classifier, policy_engine)
    }

    fn make_input(content: &str) -> GateInput {
        GateInput {
            user_id: "user-1".to_string(),
            channel_id: "telegram".to_string(),
            chat_id: "chat-1".to_string(),
            content: content.to_string(),
            content_type: ContentType::Text,
            raw_content: content.as_bytes().to_vec(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_normal_text_routes_local() {
        let gate = default_gate();
        let resource = gate.process(make_input("Hello, how are you?")).unwrap();

        assert_eq!(resource.sensitivity, SensitivityLevel::Normal);
        assert_eq!(resource.storage_location, StorageLocation::Memory);
    }

    #[test]
    fn test_sensitive_text_routes_tee() {
        let gate = default_gate();
        let resource = gate
            .process(make_input("Contact me at test@example.com"))
            .unwrap();

        assert_eq!(resource.sensitivity, SensitivityLevel::Sensitive);
        assert!(matches!(
            resource.storage_location,
            StorageLocation::Tee { .. }
        ));
    }

    #[test]
    fn test_highly_sensitive_routes_tee() {
        let gate = default_gate();
        let resource = gate
            .process(make_input("My card is 4111-1111-1111-1111"))
            .unwrap();

        assert_eq!(resource.sensitivity, SensitivityLevel::HighlySensitive);
        assert!(matches!(
            resource.storage_location,
            StorageLocation::Tee { .. }
        ));
    }

    #[test]
    fn test_classification_preserved() {
        let gate = default_gate();
        let resource = gate
            .process(make_input("SSN: 123-45-6789 and email test@example.com"))
            .unwrap();

        let classification = resource.classification.as_ref().unwrap();
        assert_eq!(classification.level, SensitivityLevel::HighlySensitive);
        assert_eq!(classification.matches.len(), 2);
        assert!(classification.requires_tee);
    }

    #[test]
    fn test_gate_with_custom_policy() {
        let classifier =
            Classifier::new(default_classification_rules(), SensitivityLevel::Normal).unwrap();

        let mut policy_engine = PolicyEngine::new();
        let strict_policy = PolicyBuilder::new("default")
            .tee_threshold(SensitivityLevel::Normal)
            .build();
        policy_engine.set_default_policy(strict_policy);

        let gate = PrivacyGate::new(classifier, policy_engine);

        // Even normal text should route to TEE with strict policy
        let resource = gate.process(make_input("Hello world")).unwrap();
        assert_eq!(resource.sensitivity, SensitivityLevel::Normal);
        assert!(matches!(
            resource.storage_location,
            StorageLocation::Tee { .. }
        ));
    }

    #[test]
    fn test_gate_reject_policy() {
        let classifier =
            Classifier::new(default_classification_rules(), SensitivityLevel::Normal).unwrap();

        let mut policy_engine = PolicyEngine::new();
        let strict_policy = PolicyBuilder::new("default")
            .allow_highly_sensitive(false)
            .build();
        policy_engine.set_default_policy(strict_policy);

        let gate = PrivacyGate::new(classifier, policy_engine);

        // Highly sensitive data with disallow policy should still create
        // resource (with Memory location as fallback)
        let resource = gate
            .process(make_input("My card is 4111-1111-1111-1111"))
            .unwrap();
        assert_eq!(resource.sensitivity, SensitivityLevel::HighlySensitive);
        assert_eq!(resource.storage_location, StorageLocation::Memory);
    }
}
