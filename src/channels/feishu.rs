//! Feishu (Lark) channel adapter

use super::adapter::{AdapterBase, AdapterStatus, ChannelAdapter, ChannelEvent};
use super::message::OutboundMessage;
use crate::config::FeishuConfig;
use crate::error::{Error, Result};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Feishu channel adapter
pub struct FeishuAdapter {
    config: FeishuConfig,
    base: AdapterBase,
    event_tx: Arc<RwLock<Option<mpsc::Sender<ChannelEvent>>>>,
}

impl FeishuAdapter {
    /// Create a new Feishu adapter
    pub fn new(config: FeishuConfig) -> Self {
        Self {
            config,
            base: AdapterBase::new("feishu"),
            event_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if a user is allowed by open_id
    pub fn is_user_allowed(&self, open_id: &str) -> bool {
        self.config.allowed_users.is_empty()
            || self.config.allowed_users.iter().any(|u| u == open_id)
    }

    /// Verify callback signature (SHA256 of timestamp + nonce + encrypt_key + body)
    pub fn verify_signature(
        timestamp: &str,
        nonce: &str,
        encrypt_key: &str,
        body: &str,
        expected: &str,
    ) -> bool {
        let content = format!("{}{}{}{}", timestamp, nonce, encrypt_key, body);
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = format!("{:x}", hasher.finalize());
        result == expected
    }
}

#[async_trait]
impl ChannelAdapter for FeishuAdapter {
    fn name(&self) -> &str {
        self.base.name()
    }

    async fn start(&self, event_tx: mpsc::Sender<ChannelEvent>) -> Result<()> {
        self.base.set_status(AdapterStatus::Starting);

        *self.event_tx.write().await = Some(event_tx.clone());

        // In a real implementation, this would:
        // 1. Obtain tenant access token via app_id + app_secret
        // 2. Register event subscription webhook
        // 3. Start processing incoming events

        tracing::info!("Feishu adapter starting (app_id={})", self.config.app_id);

        let _ = event_tx
            .send(ChannelEvent::Connected {
                channel: "feishu".to_string(),
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
                    channel: "feishu".to_string(),
                    reason: "Adapter stopped".to_string(),
                })
                .await;
        }

        *self.event_tx.write().await = None;
        self.base.set_status(AdapterStatus::Stopped);

        tracing::info!("Feishu adapter stopped");

        Ok(())
    }

    async fn send_message(&self, message: OutboundMessage) -> Result<String> {
        if !self.base.is_running() {
            return Err(Error::Channel("Feishu adapter not running".to_string()));
        }

        // In a real implementation, this would:
        // 1. Format as Feishu rich text or interactive card
        // 2. POST to Feishu API with tenant access token
        // 3. Return the message_id from response

        tracing::debug!(
            "Sending message to Feishu chat {}: {}",
            message.chat_id,
            message.content
        );

        Ok(format!("fs-msg-{}", uuid::Uuid::new_v4()))
    }

    async fn send_typing(&self, chat_id: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("Feishu adapter not running".to_string()));
        }

        tracing::debug!("Sending typing indicator to Feishu chat {}", chat_id);

        Ok(())
    }

    async fn edit_message(&self, chat_id: &str, message_id: &str, content: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("Feishu adapter not running".to_string()));
        }

        // In a real implementation, PATCH message via Feishu API
        tracing::debug!(
            "Editing Feishu message {} in chat {}: {}",
            message_id,
            chat_id,
            content
        );

        Ok(())
    }

    async fn delete_message(&self, chat_id: &str, message_id: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("Feishu adapter not running".to_string()));
        }

        // In a real implementation, DELETE message via Feishu API
        tracing::debug!("Deleting Feishu message {} in chat {}", message_id, chat_id);

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.base.is_running()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> FeishuConfig {
        FeishuConfig {
            app_id: "cli_test_app".to_string(),
            app_secret_ref: "test_secret".to_string(),
            encrypt_key_ref: "test_encrypt_key".to_string(),
            verification_token_ref: "test_token".to_string(),
            allowed_users: vec!["ou_user1".to_string(), "ou_user2".to_string()],
            dm_policy: "pairing".to_string(),
        }
    }

    #[test]
    fn test_adapter_creation() {
        let config = create_test_config();
        let adapter = FeishuAdapter::new(config);

        assert_eq!(adapter.name(), "feishu");
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_user_allowed() {
        let config = create_test_config();
        let adapter = FeishuAdapter::new(config);

        assert!(adapter.is_user_allowed("ou_user1"));
        assert!(adapter.is_user_allowed("ou_user2"));
        assert!(!adapter.is_user_allowed("ou_unknown"));
    }

    #[test]
    fn test_empty_allowed_users() {
        let config = FeishuConfig {
            allowed_users: vec![],
            ..create_test_config()
        };
        let adapter = FeishuAdapter::new(config);

        assert!(adapter.is_user_allowed("ou_anyone"));
    }

    #[test]
    fn test_verify_signature() {
        let timestamp = "1234567890";
        let nonce = "abc123";
        let encrypt_key = "mykey";
        let body = r#"{"event":"test"}"#;

        // Compute expected hash
        let content = format!("{}{}{}{}", timestamp, nonce, encrypt_key, body);
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let expected = format!("{:x}", hasher.finalize());

        assert!(FeishuAdapter::verify_signature(
            timestamp,
            nonce,
            encrypt_key,
            body,
            &expected
        ));
        assert!(!FeishuAdapter::verify_signature(
            timestamp,
            nonce,
            encrypt_key,
            body,
            "wrong_hash"
        ));
    }

    #[tokio::test]
    async fn test_adapter_lifecycle() {
        let config = create_test_config();
        let adapter = FeishuAdapter::new(config);
        let (tx, mut rx) = mpsc::channel(10);

        adapter.start(tx).await.unwrap();
        assert!(adapter.is_connected());

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, ChannelEvent::Connected { .. }));

        adapter.stop().await.unwrap();
        assert!(!adapter.is_connected());
    }
}
