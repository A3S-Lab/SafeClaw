//! Telegram channel adapter

use super::adapter::{AdapterBase, AdapterStatus, ChannelAdapter, ChannelEvent};
use super::message::OutboundMessage;
use crate::config::TelegramConfig;
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Telegram channel adapter
pub struct TelegramAdapter {
    #[allow(dead_code)]
    config: TelegramConfig,
    base: AdapterBase,
    event_tx: Arc<tokio::sync::RwLock<Option<mpsc::Sender<ChannelEvent>>>>,
}

impl TelegramAdapter {
    /// Create a new Telegram adapter
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            base: AdapterBase::new("telegram"),
            event_tx: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Check if a user is allowed
    #[allow(dead_code)]
    fn is_user_allowed(&self, user_id: i64) -> bool {
        self.config.allowed_users.is_empty() || self.config.allowed_users.contains(&user_id)
    }

    /// Handle incoming update (placeholder for actual Telegram API integration)
    #[allow(dead_code)]
    async fn handle_update(&self, _update: serde_json::Value) -> Result<()> {
        // In a real implementation, this would:
        // 1. Parse the Telegram update
        // 2. Convert to InboundMessage
        // 3. Send through event_tx

        Ok(())
    }
}

#[async_trait]
impl ChannelAdapter for TelegramAdapter {
    fn name(&self) -> &str {
        self.base.name()
    }

    async fn start(&self, event_tx: mpsc::Sender<ChannelEvent>) -> Result<()> {
        self.base.set_status(AdapterStatus::Starting);

        // Store event sender
        *self.event_tx.write().await = Some(event_tx.clone());

        // In a real implementation, this would:
        // 1. Initialize the Telegram bot API client
        // 2. Start long polling or webhook
        // 3. Begin processing updates

        tracing::info!("Telegram adapter starting");

        // Send connected event
        let _ = event_tx
            .send(ChannelEvent::Connected {
                channel: "telegram".to_string(),
            })
            .await;

        self.base.set_status(AdapterStatus::Running);

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.base.set_status(AdapterStatus::Stopping);

        // Send disconnected event
        if let Some(tx) = self.event_tx.read().await.as_ref() {
            let _ = tx
                .send(ChannelEvent::Disconnected {
                    channel: "telegram".to_string(),
                    reason: "Adapter stopped".to_string(),
                })
                .await;
        }

        *self.event_tx.write().await = None;
        self.base.set_status(AdapterStatus::Stopped);

        tracing::info!("Telegram adapter stopped");

        Ok(())
    }

    async fn send_message(&self, message: OutboundMessage) -> Result<String> {
        if !self.base.is_running() {
            return Err(Error::Channel("Telegram adapter not running".to_string()));
        }

        // In a real implementation, this would:
        // 1. Format the message for Telegram
        // 2. Send via Telegram Bot API
        // 3. Return the message ID

        tracing::debug!(
            "Sending message to Telegram chat {}: {}",
            message.chat_id,
            message.content
        );

        // Placeholder message ID
        Ok(format!("tg-msg-{}", uuid::Uuid::new_v4()))
    }

    async fn send_typing(&self, chat_id: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("Telegram adapter not running".to_string()));
        }

        // In a real implementation, send typing action via Telegram API
        tracing::debug!("Sending typing indicator to Telegram chat {}", chat_id);

        Ok(())
    }

    async fn edit_message(&self, chat_id: &str, message_id: &str, content: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("Telegram adapter not running".to_string()));
        }

        // In a real implementation, edit message via Telegram API
        tracing::debug!(
            "Editing Telegram message {} in chat {}: {}",
            message_id,
            chat_id,
            content
        );

        Ok(())
    }

    async fn delete_message(&self, chat_id: &str, message_id: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("Telegram adapter not running".to_string()));
        }

        // In a real implementation, delete message via Telegram API
        tracing::debug!(
            "Deleting Telegram message {} in chat {}",
            message_id,
            chat_id
        );

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.base.is_running()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> TelegramConfig {
        TelegramConfig {
            bot_token_ref: "test_token".to_string(),
            allowed_users: vec![123456789],
            dm_policy: "pairing".to_string(),
        }
    }

    #[test]
    fn test_adapter_creation() {
        let config = create_test_config();
        let adapter = TelegramAdapter::new(config);

        assert_eq!(adapter.name(), "telegram");
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_user_allowed() {
        let config = create_test_config();
        let adapter = TelegramAdapter::new(config);

        assert!(adapter.is_user_allowed(123456789));
        assert!(!adapter.is_user_allowed(987654321));
    }

    #[test]
    fn test_empty_allowed_users() {
        let config = TelegramConfig {
            bot_token_ref: "test_token".to_string(),
            allowed_users: vec![],
            dm_policy: "open".to_string(),
        };
        let adapter = TelegramAdapter::new(config);

        // Empty list means all users allowed
        assert!(adapter.is_user_allowed(123456789));
        assert!(adapter.is_user_allowed(987654321));
    }

    #[tokio::test]
    async fn test_adapter_lifecycle() {
        let config = create_test_config();
        let adapter = TelegramAdapter::new(config);
        let (tx, mut rx) = mpsc::channel(10);

        adapter.start(tx).await.unwrap();
        assert!(adapter.is_connected());

        // Should receive connected event
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, ChannelEvent::Connected { .. }));

        adapter.stop().await.unwrap();
        assert!(!adapter.is_connected());
    }
}
