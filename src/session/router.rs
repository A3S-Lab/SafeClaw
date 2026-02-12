//! Session routing based on privacy classification

use crate::channels::InboundMessage;
use crate::error::Result;
use crate::privacy::{ClassificationResult, Classifier, PolicyDecision, PolicyEngine};
use crate::session::SessionManager;
use std::sync::Arc;

/// Routing decision for a message
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// Session to route to
    pub session_id: String,
    /// Whether to process in TEE
    pub use_tee: bool,
    /// Classification result
    pub classification: ClassificationResult,
    /// Policy decision
    pub policy_decision: PolicyDecision,
}

/// Session router that handles message routing based on privacy
pub struct SessionRouter {
    session_manager: Arc<SessionManager>,
    classifier: Arc<Classifier>,
    policy_engine: Arc<PolicyEngine>,
}

impl SessionRouter {
    /// Create a new session router
    pub fn new(
        session_manager: Arc<SessionManager>,
        classifier: Arc<Classifier>,
        policy_engine: Arc<PolicyEngine>,
    ) -> Self {
        Self {
            session_manager,
            classifier,
            policy_engine,
        }
    }

    /// Route an inbound message
    pub async fn route(&self, message: &InboundMessage) -> Result<RoutingDecision> {
        // Classify the message content
        let classification = self.classifier.classify(&message.content);

        // Get policy decision
        let policy_decision = self.policy_engine.evaluate(
            classification.level,
            None, // No specific data type
            None, // Use default policy
        );

        // Determine if TEE is needed
        let use_tee = matches!(policy_decision, PolicyDecision::ProcessInTee)
            && self.session_manager.is_tee_enabled();

        // Get or create session
        let session = self
            .session_manager
            .get_user_session(&message.sender_id, &message.channel, &message.chat_id)
            .await;

        let session = match session {
            Some(s) => s,
            None => {
                self.session_manager
                    .create_session(&message.sender_id, &message.channel, &message.chat_id)
                    .await?
            }
        };

        // Update session sensitivity level
        session.update_sensitivity(classification.level).await;

        // If TEE is needed and session doesn't have TEE, upgrade it
        if use_tee && !session.uses_tee().await {
            self.session_manager.upgrade_to_tee(&session.id).await?;
        }

        // Update session activity
        session.touch().await;
        session.increment_messages().await;

        Ok(RoutingDecision {
            session_id: session.id.clone(),
            use_tee,
            classification,
            policy_decision,
        })
    }

    /// Check if a message requires TEE processing
    pub fn requires_tee(&self, message: &InboundMessage) -> bool {
        let classification = self.classifier.classify(&message.content);
        self.policy_engine.requires_tee(classification.level)
            && self.session_manager.is_tee_enabled()
    }

    /// Get redacted version of message content
    pub fn redact_content(&self, content: &str) -> String {
        self.classifier.redact(content)
    }

    /// Get classification for content
    pub fn classify(&self, content: &str) -> ClassificationResult {
        self.classifier.classify(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PrivacyConfig, SensitivityLevel, TeeConfig};
    use crate::privacy::Classifier;

    fn create_test_router() -> SessionRouter {
        // Disable TEE for tests to avoid connection requirements
        let tee_config = TeeConfig {
            enabled: false,
            ..Default::default()
        };
        let session_manager = Arc::new(SessionManager::new(tee_config));
        let privacy_config = PrivacyConfig::default();
        let classifier =
            Arc::new(Classifier::new(privacy_config.rules, privacy_config.default_level).unwrap());
        let policy_engine = Arc::new(PolicyEngine::new());

        SessionRouter::new(session_manager, classifier, policy_engine)
    }

    #[tokio::test]
    async fn test_route_normal_message() {
        let router = create_test_router();
        let message =
            InboundMessage::new("telegram", "user-123", "chat-456", "Hello, how are you?");

        let decision = router.route(&message).await.unwrap();

        assert!(!decision.use_tee);
        assert_eq!(decision.classification.level, SensitivityLevel::Normal);
    }

    #[tokio::test]
    async fn test_route_sensitive_message() {
        let router = create_test_router();
        let message = InboundMessage::new(
            "telegram",
            "user-123",
            "chat-456",
            "My credit card is 4111-1111-1111-1111",
        );

        let decision = router.route(&message).await.unwrap();

        // TEE is disabled in test, so use_tee should be false
        assert!(!decision.use_tee);
        // But classification should still detect highly sensitive data
        assert_eq!(
            decision.classification.level,
            SensitivityLevel::HighlySensitive
        );
    }

    #[test]
    fn test_requires_tee() {
        let router = create_test_router();

        let normal_msg = InboundMessage::new("telegram", "user-123", "chat-456", "Hello!");
        assert!(!router.requires_tee(&normal_msg));

        // TEE is disabled in test router, so requires_tee always returns false
        let sensitive_msg =
            InboundMessage::new("telegram", "user-123", "chat-456", "My SSN is 123-45-6789");
        // With TEE disabled, requires_tee returns false even for sensitive data
        assert!(!router.requires_tee(&sensitive_msg));
    }

    #[test]
    fn test_redact_content() {
        let router = create_test_router();

        let content = "My email is test@example.com and SSN is 123-45-6789";
        let redacted = router.redact_content(content);

        assert!(!redacted.contains("test@example.com"));
        assert!(!redacted.contains("123-45-6789"));
        assert!(redacted.contains("****@example.com"));
        assert!(redacted.contains("***-**-****"));
    }
}
