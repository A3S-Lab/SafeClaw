//! WeCom (WeChat Work) channel adapter

use super::adapter::{AdapterBase, AdapterStatus, ChannelAdapter, ChannelEvent};
use super::message::OutboundMessage;
use crate::config::WeComConfig;
use crate::error::{Error, Result};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// WeCom channel adapter
pub struct WeComAdapter {
    config: WeComConfig,
    base: AdapterBase,
    event_tx: Arc<RwLock<Option<mpsc::Sender<ChannelEvent>>>>,
}

impl WeComAdapter {
    /// Create a new WeCom adapter
    pub fn new(config: WeComConfig) -> Self {
        Self {
            config,
            base: AdapterBase::new("wecom"),
            event_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if a user is allowed by userId
    pub fn is_user_allowed(&self, user_id: &str) -> bool {
        self.config.allowed_users.is_empty()
            || self.config.allowed_users.iter().any(|u| u == user_id)
    }

    /// Verify callback URL (sort token, timestamp, nonce; SHA-256 hash; compare with signature)
    pub fn verify_callback(token: &str, timestamp: &str, nonce: &str, signature: &str) -> bool {
        let mut parts = vec![token, timestamp, nonce];
        parts.sort();
        let combined = parts.join("");

        let mut hasher = Sha256::new();
        hasher.update(combined.as_bytes());
        let result = format!("{:x}", hasher.finalize());
        result == signature
    }
}

#[async_trait]
impl ChannelAdapter for WeComAdapter {
    fn name(&self) -> &str {
        self.base.name()
    }

    async fn start(&self, event_tx: mpsc::Sender<ChannelEvent>) -> Result<()> {
        self.base.set_status(AdapterStatus::Starting);

        *self.event_tx.write().await = Some(event_tx.clone());

        // In a real implementation, this would:
        // 1. Obtain access token via corp_id + corp_secret
        // 2. Register application message callback URL
        // 3. Start processing incoming events (decrypt AES-256-CBC with EncodingAESKey)

        tracing::info!(
            "WeCom adapter starting (corp_id={}, agent_id={})",
            self.config.corp_id,
            self.config.agent_id
        );

        let _ = event_tx
            .send(ChannelEvent::Connected {
                channel: "wecom".to_string(),
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
                    channel: "wecom".to_string(),
                    reason: "Adapter stopped".to_string(),
                })
                .await;
        }

        *self.event_tx.write().await = None;
        self.base.set_status(AdapterStatus::Stopped);

        tracing::info!("WeCom adapter stopped");

        Ok(())
    }

    async fn send_message(&self, message: OutboundMessage) -> Result<String> {
        if !self.base.is_running() {
            return Err(Error::Channel("WeCom adapter not running".to_string()));
        }

        // In a real implementation, this would:
        // 1. Format as WeCom text or markdown message
        // 2. POST to WeCom API with access token
        // 3. Return the msgid from response

        tracing::debug!(
            "Sending message to WeCom user {}: {}",
            message.chat_id,
            message.content
        );

        Ok(format!("wc-msg-{}", uuid::Uuid::new_v4()))
    }

    async fn send_typing(&self, chat_id: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("WeCom adapter not running".to_string()));
        }

        // WeCom does not natively support typing indicators
        tracing::debug!(
            "Typing indicator not supported for WeCom chat {}",
            chat_id
        );

        Ok(())
    }

    async fn edit_message(&self, _chat_id: &str, _message_id: &str, _content: &str) -> Result<()> {
        Err(Error::Channel(
            "Message editing not supported for WeCom".to_string(),
        ))
    }

    async fn delete_message(&self, _chat_id: &str, _message_id: &str) -> Result<()> {
        Err(Error::Channel(
            "Message deletion not supported for WeCom".to_string(),
        ))
    }

    fn is_connected(&self) -> bool {
        self.base.is_running()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> WeComConfig {
        WeComConfig {
            corp_id: "ww_test_corp".to_string(),
            agent_id: 1000001,
            secret_ref: "test_secret".to_string(),
            encoding_aes_key_ref: "test_aes_key".to_string(),
            token_ref: "test_token".to_string(),
            allowed_users: vec!["user001".to_string(), "user002".to_string()],
            dm_policy: "pairing".to_string(),
        }
    }

    #[test]
    fn test_adapter_creation() {
        let config = create_test_config();
        let adapter = WeComAdapter::new(config);

        assert_eq!(adapter.name(), "wecom");
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_user_allowed() {
        let config = create_test_config();
        let adapter = WeComAdapter::new(config);

        assert!(adapter.is_user_allowed("user001"));
        assert!(adapter.is_user_allowed("user002"));
        assert!(!adapter.is_user_allowed("user999"));
    }

    #[test]
    fn test_empty_allowed_users() {
        let config = WeComConfig {
            allowed_users: vec![],
            ..create_test_config()
        };
        let adapter = WeComAdapter::new(config);

        assert!(adapter.is_user_allowed("anyone"));
    }

    #[test]
    fn test_verify_callback() {
        let token = "mytoken";
        let timestamp = "1234567890";
        let nonce = "abc123";

        // Compute expected signature
        let mut parts = vec![token, timestamp, nonce];
        parts.sort();
        let combined = parts.join("");
        let mut hasher = Sha256::new();
        hasher.update(combined.as_bytes());
        let expected = format!("{:x}", hasher.finalize());

        assert!(WeComAdapter::verify_callback(
            token, timestamp, nonce, &expected
        ));
        assert!(!WeComAdapter::verify_callback(
            token, timestamp, nonce, "wrong"
        ));
    }

    #[tokio::test]
    async fn test_adapter_lifecycle() {
        let config = create_test_config();
        let adapter = WeComAdapter::new(config);
        let (tx, mut rx) = mpsc::channel(10);

        adapter.start(tx).await.unwrap();
        assert!(adapter.is_connected());

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, ChannelEvent::Connected { .. }));

        adapter.stop().await.unwrap();
        assert!(!adapter.is_connected());
    }
}
