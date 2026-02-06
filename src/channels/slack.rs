//! Slack channel adapter

use super::adapter::{AdapterBase, AdapterStatus, ChannelAdapter, ChannelEvent};
use super::message::OutboundMessage;
use crate::config::SlackConfig;
use crate::error::{Error, Result};
use async_trait::async_trait;
use ring::hmac;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Slack channel adapter
pub struct SlackAdapter {
    config: SlackConfig,
    base: AdapterBase,
    event_tx: Arc<RwLock<Option<mpsc::Sender<ChannelEvent>>>>,
}

impl SlackAdapter {
    /// Create a new Slack adapter
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            base: AdapterBase::new("slack"),
            event_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if a workspace is allowed
    pub fn is_workspace_allowed(&self, workspace_id: &str) -> bool {
        self.config.allowed_workspaces.is_empty()
            || self
                .config
                .allowed_workspaces
                .iter()
                .any(|w| w == workspace_id)
    }

    /// Verify request signature (HMAC-SHA256 with signing_secret)
    /// Slack signs requests with: v0=HMAC-SHA256(signing_secret, "v0:{timestamp}:{body}")
    pub fn verify_signature(
        signing_secret: &str,
        timestamp: &str,
        body: &str,
        expected: &str,
    ) -> bool {
        let sig_basestring = format!("v0:{}:{}", timestamp, body);
        let key = hmac::Key::new(hmac::HMAC_SHA256, signing_secret.as_bytes());
        let signature = hmac::sign(&key, sig_basestring.as_bytes());
        let computed = format!("v0={}", hex::encode(signature.as_ref()));
        computed == expected
    }
}

#[async_trait]
impl ChannelAdapter for SlackAdapter {
    fn name(&self) -> &str {
        self.base.name()
    }

    async fn start(&self, event_tx: mpsc::Sender<ChannelEvent>) -> Result<()> {
        self.base.set_status(AdapterStatus::Starting);

        *self.event_tx.write().await = Some(event_tx.clone());

        // In a real implementation, this would:
        // 1. Connect via Socket Mode (app_token) or register Events API webhook
        // 2. Authenticate with bot_token
        // 3. Start processing incoming events

        tracing::info!("Slack adapter starting");

        let _ = event_tx
            .send(ChannelEvent::Connected {
                channel: "slack".to_string(),
            })
            .await;

        self.base.set_status(AdapterStatus::Running);

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.base.set_status(AdapterStatus::Stopping);

        if let Some(tx) = self.event_tx.read().await.as_ref() {
            let _ = tx
                .send(ChannelEvent::Disconnected {
                    channel: "slack".to_string(),
                    reason: "Adapter stopped".to_string(),
                })
                .await;
        }

        *self.event_tx.write().await = None;
        self.base.set_status(AdapterStatus::Stopped);

        tracing::info!("Slack adapter stopped");

        Ok(())
    }

    async fn send_message(&self, message: OutboundMessage) -> Result<String> {
        if !self.base.is_running() {
            return Err(Error::Channel("Slack adapter not running".to_string()));
        }

        // In a real implementation, this would:
        // 1. Format as Slack Block Kit message
        // 2. POST to chat.postMessage API with bot_token
        // 3. Support thread replies via thread_ts
        // 4. Return the ts (message ID) from response

        tracing::debug!(
            "Sending message to Slack channel {}: {}",
            message.chat_id,
            message.content
        );

        Ok(format!("sl-msg-{}", uuid::Uuid::new_v4()))
    }

    async fn send_typing(&self, chat_id: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("Slack adapter not running".to_string()));
        }

        // Slack typing is handled via Socket Mode
        tracing::debug!("Sending typing indicator to Slack channel {}", chat_id);

        Ok(())
    }

    async fn edit_message(&self, chat_id: &str, message_id: &str, content: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("Slack adapter not running".to_string()));
        }

        // In a real implementation, call chat.update API
        tracing::debug!(
            "Editing Slack message {} in channel {}: {}",
            message_id,
            chat_id,
            content
        );

        Ok(())
    }

    async fn delete_message(&self, chat_id: &str, message_id: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("Slack adapter not running".to_string()));
        }

        // In a real implementation, call chat.delete API
        tracing::debug!(
            "Deleting Slack message {} in channel {}",
            message_id,
            chat_id
        );

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.base.is_running()
    }
}

/// Hex encoding helper (avoids adding hex crate dependency)
mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> SlackConfig {
        SlackConfig {
            bot_token_ref: "xoxb-test-token".to_string(),
            app_token_ref: "xapp-test-token".to_string(),
            allowed_workspaces: vec!["T01234567".to_string()],
            dm_policy: "pairing".to_string(),
        }
    }

    #[test]
    fn test_adapter_creation() {
        let config = create_test_config();
        let adapter = SlackAdapter::new(config);

        assert_eq!(adapter.name(), "slack");
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_workspace_allowed() {
        let config = create_test_config();
        let adapter = SlackAdapter::new(config);

        assert!(adapter.is_workspace_allowed("T01234567"));
        assert!(!adapter.is_workspace_allowed("T99999999"));
    }

    #[test]
    fn test_empty_allowed_workspaces() {
        let config = SlackConfig {
            allowed_workspaces: vec![],
            ..create_test_config()
        };
        let adapter = SlackAdapter::new(config);

        assert!(adapter.is_workspace_allowed("any_workspace"));
    }

    #[test]
    fn test_verify_signature() {
        let signing_secret = "test_signing_secret";
        let timestamp = "1531420618";
        let body = r#"token=xyzz0WbapA4vBCDEFasx0q6G&team_id=T1DC2JH3J"#;

        // Compute expected signature
        let sig_basestring = format!("v0:{}:{}", timestamp, body);
        let key = hmac::Key::new(hmac::HMAC_SHA256, signing_secret.as_bytes());
        let signature = hmac::sign(&key, sig_basestring.as_bytes());
        let expected = format!("v0={}", hex::encode(signature.as_ref()));

        assert!(SlackAdapter::verify_signature(
            signing_secret,
            timestamp,
            body,
            &expected
        ));
        assert!(!SlackAdapter::verify_signature(
            signing_secret,
            timestamp,
            body,
            "v0=wrong"
        ));
    }

    #[tokio::test]
    async fn test_adapter_lifecycle() {
        let config = create_test_config();
        let adapter = SlackAdapter::new(config);
        let (tx, mut rx) = mpsc::channel(10);

        adapter.start(tx).await.unwrap();
        assert!(adapter.is_connected());

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, ChannelEvent::Connected { .. }));

        adapter.stop().await.unwrap();
        assert!(!adapter.is_connected());
    }
}
