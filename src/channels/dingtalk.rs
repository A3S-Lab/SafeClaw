//! DingTalk channel adapter

use super::adapter::{AdapterBase, AdapterStatus, ChannelAdapter, ChannelEvent};
use super::message::OutboundMessage;
use crate::config::DingTalkConfig;
use crate::error::{Error, Result};
use async_trait::async_trait;
use ring::hmac;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// DingTalk channel adapter
pub struct DingTalkAdapter {
    config: DingTalkConfig,
    base: AdapterBase,
    event_tx: Arc<RwLock<Option<mpsc::Sender<ChannelEvent>>>>,
}

impl DingTalkAdapter {
    /// Create a new DingTalk adapter
    pub fn new(config: DingTalkConfig) -> Self {
        Self {
            config,
            base: AdapterBase::new("dingtalk"),
            event_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if a user is allowed by staffId
    pub fn is_user_allowed(&self, staff_id: &str) -> bool {
        self.config.allowed_users.is_empty()
            || self.config.allowed_users.iter().any(|u| u == staff_id)
    }

    /// Verify callback signature (HMAC-SHA256 of timestamp + secret)
    pub fn verify_signature(timestamp: &str, secret: &str, expected: &str) -> bool {
        let string_to_sign = format!("{}\n{}", timestamp, secret);
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
        let signature = hmac::sign(&key, string_to_sign.as_bytes());
        use base64::Engine as _;
        let encoded = base64::engine::general_purpose::STANDARD.encode(signature.as_ref());
        encoded == expected
    }
}

#[async_trait]
impl ChannelAdapter for DingTalkAdapter {
    fn name(&self) -> &str {
        self.base.name()
    }

    async fn start(&self, event_tx: mpsc::Sender<ChannelEvent>) -> Result<()> {
        self.base.set_status(AdapterStatus::Starting);

        *self.event_tx.write().await = Some(event_tx.clone());

        // In a real implementation, this would:
        // 1. Register robot callback URL
        // 2. Start processing incoming robot messages
        // 3. Obtain access token via app_key + app_secret

        tracing::info!(
            "DingTalk adapter starting (robot_code={})",
            self.config.robot_code
        );

        let _ = event_tx
            .send(ChannelEvent::Connected {
                channel: "dingtalk".to_string(),
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
                    channel: "dingtalk".to_string(),
                    reason: "Adapter stopped".to_string(),
                })
                .await;
        }

        *self.event_tx.write().await = None;
        self.base.set_status(AdapterStatus::Stopped);

        tracing::info!("DingTalk adapter stopped");

        Ok(())
    }

    async fn send_message(&self, message: OutboundMessage) -> Result<String> {
        if !self.base.is_running() {
            return Err(Error::Channel("DingTalk adapter not running".to_string()));
        }

        // In a real implementation, this would:
        // 1. Format as DingTalk markdown or text message
        // 2. POST to robot webhook URL
        // 3. Return the message key from response

        tracing::debug!(
            "Sending message to DingTalk chat {}: {}",
            message.chat_id,
            message.content
        );

        Ok(format!("dt-msg-{}", uuid::Uuid::new_v4()))
    }

    async fn send_typing(&self, chat_id: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("DingTalk adapter not running".to_string()));
        }

        // DingTalk does not natively support typing indicators
        tracing::debug!(
            "Typing indicator not supported for DingTalk chat {}",
            chat_id
        );

        Ok(())
    }

    async fn edit_message(&self, _chat_id: &str, _message_id: &str, _content: &str) -> Result<()> {
        Err(Error::Channel(
            "Message editing not supported for DingTalk".to_string(),
        ))
    }

    async fn delete_message(&self, _chat_id: &str, _message_id: &str) -> Result<()> {
        Err(Error::Channel(
            "Message deletion not supported for DingTalk".to_string(),
        ))
    }

    fn is_connected(&self) -> bool {
        self.base.is_running()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> DingTalkConfig {
        DingTalkConfig {
            app_key_ref: "test_key".to_string(),
            app_secret_ref: "test_secret".to_string(),
            robot_code: "robot_test".to_string(),
            allowed_users: vec!["staff001".to_string(), "staff002".to_string()],
            dm_policy: "pairing".to_string(),
        }
    }

    #[test]
    fn test_adapter_creation() {
        let config = create_test_config();
        let adapter = DingTalkAdapter::new(config);

        assert_eq!(adapter.name(), "dingtalk");
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_user_allowed() {
        let config = create_test_config();
        let adapter = DingTalkAdapter::new(config);

        assert!(adapter.is_user_allowed("staff001"));
        assert!(adapter.is_user_allowed("staff002"));
        assert!(!adapter.is_user_allowed("staff999"));
    }

    #[test]
    fn test_empty_allowed_users() {
        let config = DingTalkConfig {
            allowed_users: vec![],
            ..create_test_config()
        };
        let adapter = DingTalkAdapter::new(config);

        assert!(adapter.is_user_allowed("anyone"));
    }

    #[test]
    fn test_verify_signature() {
        let timestamp = "1234567890";
        let secret = "mysecret";

        // Compute expected signature
        let string_to_sign = format!("{}\n{}", timestamp, secret);
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
        let signature = hmac::sign(&key, string_to_sign.as_bytes());
        use base64::Engine as _;
        let expected = base64::engine::general_purpose::STANDARD.encode(signature.as_ref());

        assert!(DingTalkAdapter::verify_signature(
            timestamp, secret, &expected
        ));
        assert!(!DingTalkAdapter::verify_signature(
            timestamp, secret, "wrong"
        ));
    }

    #[tokio::test]
    async fn test_adapter_lifecycle() {
        let config = create_test_config();
        let adapter = DingTalkAdapter::new(config);
        let (tx, mut rx) = mpsc::channel(10);

        adapter.start(tx).await.unwrap();
        assert!(adapter.is_connected());

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, ChannelEvent::Connected { .. }));

        adapter.stop().await.unwrap();
        assert!(!adapter.is_connected());
    }
}
