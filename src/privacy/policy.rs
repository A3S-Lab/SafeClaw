//! Privacy policy engine for data routing decisions

use crate::config::SensitivityLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Policy decision for data handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    /// Process locally (no TEE required)
    ProcessLocal,
    /// Process in TEE environment
    ProcessInTee,
    /// Reject processing entirely
    Reject,
    /// Require user confirmation before processing
    RequireConfirmation,
}

/// Data policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPolicy {
    /// Policy name
    pub name: String,
    /// Minimum sensitivity level that triggers TEE processing
    pub tee_threshold: SensitivityLevel,
    /// Whether to allow processing of highly sensitive data
    pub allow_highly_sensitive: bool,
    /// Custom rules by data type
    pub type_rules: HashMap<String, PolicyDecision>,
}

impl Default for DataPolicy {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            tee_threshold: SensitivityLevel::Sensitive,
            allow_highly_sensitive: true,
            type_rules: HashMap::new(),
        }
    }
}

/// Policy engine for making data routing decisions
pub struct PolicyEngine {
    policies: HashMap<String, DataPolicy>,
    default_policy: DataPolicy,
}

impl PolicyEngine {
    /// Create a new policy engine with default policy
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            default_policy: DataPolicy::default(),
        }
    }

    /// Add a named policy
    pub fn add_policy(&mut self, policy: DataPolicy) {
        self.policies.insert(policy.name.clone(), policy);
    }

    /// Set the default policy
    pub fn set_default_policy(&mut self, policy: DataPolicy) {
        self.default_policy = policy;
    }

    /// Evaluate a policy decision for given sensitivity level
    pub fn evaluate(
        &self,
        level: SensitivityLevel,
        data_type: Option<&str>,
        policy_name: Option<&str>,
    ) -> PolicyDecision {
        let policy = policy_name
            .and_then(|name| self.policies.get(name))
            .unwrap_or(&self.default_policy);

        // Check custom type rules first
        if let Some(data_type) = data_type {
            if let Some(decision) = policy.type_rules.get(data_type) {
                return *decision;
            }
        }

        // Check if highly sensitive data is allowed
        if level == SensitivityLevel::HighlySensitive && !policy.allow_highly_sensitive {
            return PolicyDecision::Reject;
        }

        // Determine based on threshold
        match level {
            SensitivityLevel::Public => PolicyDecision::ProcessLocal,
            SensitivityLevel::Normal => {
                if policy.tee_threshold == SensitivityLevel::Normal {
                    PolicyDecision::ProcessInTee
                } else {
                    PolicyDecision::ProcessLocal
                }
            }
            SensitivityLevel::Sensitive => {
                if policy.tee_threshold <= SensitivityLevel::Sensitive {
                    PolicyDecision::ProcessInTee
                } else {
                    PolicyDecision::ProcessLocal
                }
            }
            SensitivityLevel::HighlySensitive => PolicyDecision::ProcessInTee,
            SensitivityLevel::Critical => PolicyDecision::ProcessInTee,
        }
    }

    /// Check if TEE is required for given sensitivity level
    pub fn requires_tee(&self, level: SensitivityLevel) -> bool {
        matches!(
            self.evaluate(level, None, None),
            PolicyDecision::ProcessInTee
        )
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating custom policies
#[allow(dead_code)]
pub struct PolicyBuilder {
    policy: DataPolicy,
}

#[allow(dead_code)]
impl PolicyBuilder {
    /// Create a new policy builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            policy: DataPolicy {
                name: name.into(),
                ..Default::default()
            },
        }
    }

    /// Set the TEE threshold
    pub fn tee_threshold(mut self, level: SensitivityLevel) -> Self {
        self.policy.tee_threshold = level;
        self
    }

    /// Set whether highly sensitive data is allowed
    pub fn allow_highly_sensitive(mut self, allow: bool) -> Self {
        self.policy.allow_highly_sensitive = allow;
        self
    }

    /// Add a custom rule for a data type
    pub fn add_type_rule(mut self, data_type: impl Into<String>, decision: PolicyDecision) -> Self {
        self.policy.type_rules.insert(data_type.into(), decision);
        self
    }

    /// Build the policy
    pub fn build(self) -> DataPolicy {
        self.policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy() {
        let engine = PolicyEngine::new();

        assert_eq!(
            engine.evaluate(SensitivityLevel::Public, None, None),
            PolicyDecision::ProcessLocal
        );
        assert_eq!(
            engine.evaluate(SensitivityLevel::Normal, None, None),
            PolicyDecision::ProcessLocal
        );
        assert_eq!(
            engine.evaluate(SensitivityLevel::Sensitive, None, None),
            PolicyDecision::ProcessInTee
        );
        assert_eq!(
            engine.evaluate(SensitivityLevel::HighlySensitive, None, None),
            PolicyDecision::ProcessInTee
        );
    }

    #[test]
    fn test_custom_policy() {
        let mut engine = PolicyEngine::new();

        let strict_policy = PolicyBuilder::new("strict")
            .tee_threshold(SensitivityLevel::Normal)
            .allow_highly_sensitive(false)
            .build();

        engine.add_policy(strict_policy);

        assert_eq!(
            engine.evaluate(SensitivityLevel::Normal, None, Some("strict")),
            PolicyDecision::ProcessInTee
        );
        assert_eq!(
            engine.evaluate(SensitivityLevel::HighlySensitive, None, Some("strict")),
            PolicyDecision::Reject
        );
    }

    #[test]
    fn test_type_rules() {
        let mut engine = PolicyEngine::new();

        let policy = PolicyBuilder::new("custom")
            .add_type_rule("api_key", PolicyDecision::Reject)
            .add_type_rule("email", PolicyDecision::RequireConfirmation)
            .build();

        engine.add_policy(policy);

        assert_eq!(
            engine.evaluate(SensitivityLevel::Sensitive, Some("api_key"), Some("custom")),
            PolicyDecision::Reject
        );
        assert_eq!(
            engine.evaluate(SensitivityLevel::Sensitive, Some("email"), Some("custom")),
            PolicyDecision::RequireConfirmation
        );
    }

    #[test]
    fn test_requires_tee() {
        let engine = PolicyEngine::new();

        assert!(!engine.requires_tee(SensitivityLevel::Public));
        assert!(!engine.requires_tee(SensitivityLevel::Normal));
        assert!(engine.requires_tee(SensitivityLevel::Sensitive));
        assert!(engine.requires_tee(SensitivityLevel::HighlySensitive));
    }
}
