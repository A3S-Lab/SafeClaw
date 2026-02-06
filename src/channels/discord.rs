//! Discord channel adapter

use super::adapter::{AdapterBase, AdapterStatus, ChannelAdapter, ChannelEvent};
use super::message::OutboundMessage;
use crate::config::DiscordConfig;
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Discord channel adapter
pub struct DiscordAdapter {
    config: DiscordConfig,
    base: AdapterBase,
    event_tx: Arc<RwLock<Option<mpsc::Sender<ChannelEvent>>>>,
}

impl DiscordAdapter {
    /// Create a new Discord adapter
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            base: AdapterBase::new("discord"),
            event_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if a guild is allowed
    pub fn is_guild_allowed(&self, guild_id: u64) -> bool {
        self.config.allowed_guilds.is_empty() || self.config.allowed_guilds.contains(&guild_id)
    }
}

#[async_trait]
impl ChannelAdapter for DiscordAdapter {
    fn name(&self) -> &str {
        self.base.name()
    }

    async fn start(&self, event_tx: mpsc::Sender<ChannelEvent>) -> Result<()> {
        self.base.set_status(AdapterStatus::Starting);

        *self.event_tx.write().await = Some(event_tx.clone());

        // In a real implementation, this would:
        // 1. Connect to Discord Gateway via WebSocket
        // 2. Authenticate with bot token
        // 3. Start processing incoming events (MESSAGE_CREATE, etc.)

        tracing::info!("Discord adapter starting");

        let _ = event_tx
            .send(ChannelEvent::Connected {
                channel: "discord".to_string(),
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
                    channel: "discord".to_string(),
                    reason: "Adapter stopped".to_string(),
                })
                .await;
        }

        *self.event_tx.write().await = None;
        self.base.set_status(AdapterStatus::Stopped);

        tracing::info!("Discord adapter stopped");

        Ok(())
    }

    async fn send_message(&self, message: OutboundMessage) -> Result<String> {
        if !self.base.is_running() {
            return Err(Error::Channel("Discord adapter not running".to_string()));
        }

        // In a real implementation, this would:
        // 1. Format as Discord embed or plain message
        // 2. POST to channels/{channel_id}/messages API with bot token
        // 3. Support thread/channel replies
        // 4. Return the message ID from response

        tracing::debug!(
            "Sending message to Discord channel {}: {}",
            message.chat_id,
            message.content
        );

        Ok(format!("dc-msg-{}", uuid::Uuid::new_v4()))
    }

    async fn send_typing(&self, chat_id: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("Discord adapter not running".to_string()));
        }

        // In a real implementation, POST to channels/{channel_id}/typing
        tracing::debug!("Sending typing indicator to Discord channel {}", chat_id);

        Ok(())
    }

    async fn edit_message(&self, chat_id: &str, message_id: &str, content: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("Discord adapter not running".to_string()));
        }

        // In a real implementation, PATCH channels/{channel_id}/messages/{message_id}
        tracing::debug!(
            "Editing Discord message {} in channel {}: {}",
            message_id,
            chat_id,
            content
        );

        Ok(())
    }

    async fn delete_message(&self, chat_id: &str, message_id: &str) -> Result<()> {
        if !self.base.is_running() {
            return Err(Error::Channel("Discord adapter not running".to_string()));
        }

        // In a real implementation, DELETE channels/{channel_id}/messages/{message_id}
        tracing::debug!(
            "Deleting Discord message {} in channel {}",
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

    fn create_test_config() -> DiscordConfig {
        DiscordConfig {
            bot_token_ref: "test_bot_token".to_string(),
            allowed_guilds: vec![123456789012345678],
            dm_policy: "pairing".to_string(),
        }
    }

    #[test]
    fn test_adapter_creation() {
        let config = create_test_config();
        let adapter = DiscordAdapter::new(config);

        assert_eq!(adapter.name(), "discord");
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_guild_allowed() {
        let config = create_test_config();
        let adapter = DiscordAdapter::new(config);

        assert!(adapter.is_guild_allowed(123456789012345678));
        assert!(!adapter.is_guild_allowed(999999999999999999));
    }

    #[test]
    fn test_empty_allowed_guilds() {
        let config = DiscordConfig {
            allowed_guilds: vec![],
            ..create_test_config()
        };
        let adapter = DiscordAdapter::new(config);

        assert!(adapter.is_guild_allowed(123456789012345678));
        assert!(adapter.is_guild_allowed(999999999999999999));
    }

    #[tokio::test]
    async fn test_adapter_lifecycle() {
        let config = create_test_config();
        let adapter = DiscordAdapter::new(config);
        let (tx, mut rx) = mpsc::channel(10);

        adapter.start(tx).await.unwrap();
        assert!(adapter.is_connected());

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, ChannelEvent::Connected { .. }));

        adapter.stop().await.unwrap();
        assert!(!adapter.is_connected());
    }
}
