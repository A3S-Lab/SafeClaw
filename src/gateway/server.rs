//! Gateway server implementation

use crate::channels::{
    ChannelAdapter, ChannelEvent, DingTalkAdapter, DiscordAdapter, FeishuAdapter, SlackAdapter,
    TelegramAdapter, WebChatAdapter, WeComAdapter,
};
use crate::config::SafeClawConfig;
use crate::error::{Error, Result};
use crate::privacy::{Classifier, PolicyEngine};
use crate::session::{SessionManager, SessionRouter};
use crate::tee::TeeManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Gateway server state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayState {
    /// Not started
    Stopped,
    /// Starting up
    Starting,
    /// Running
    Running,
    /// Shutting down
    ShuttingDown,
}

/// SafeClaw Gateway server
pub struct Gateway {
    config: SafeClawConfig,
    state: Arc<RwLock<GatewayState>>,
    session_manager: Arc<SessionManager>,
    tee_manager: Arc<TeeManager>,
    session_router: Arc<SessionRouter>,
    channels: Arc<RwLock<HashMap<String, Arc<dyn ChannelAdapter>>>>,
    event_tx: mpsc::Sender<ChannelEvent>,
    event_rx: Arc<RwLock<Option<mpsc::Receiver<ChannelEvent>>>>,
}

impl Gateway {
    /// Create a new gateway with the given configuration
    pub fn new(config: SafeClawConfig) -> Self {
        let (event_tx, event_rx) = mpsc::channel(1000);

        let session_manager = Arc::new(SessionManager::new());
        let tee_manager = Arc::new(TeeManager::new(config.tee.clone()));

        let classifier = Arc::new(
            Classifier::new(
                config.privacy.rules.clone(),
                config.privacy.default_level,
            )
            .expect("Failed to create classifier"),
        );
        let policy_engine = Arc::new(PolicyEngine::new());

        let session_router = Arc::new(SessionRouter::new(
            session_manager.clone(),
            tee_manager.clone(),
            classifier,
            policy_engine,
        ));

        Self {
            config,
            state: Arc::new(RwLock::new(GatewayState::Stopped)),
            session_manager,
            tee_manager,
            session_router,
            channels: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
        }
    }

    /// Get current state
    pub async fn state(&self) -> GatewayState {
        *self.state.read().await
    }

    /// Start the gateway
    pub async fn start(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if *state != GatewayState::Stopped {
            return Err(Error::Gateway("Gateway already running".to_string()));
        }
        *state = GatewayState::Starting;
        drop(state);

        tracing::info!("Starting SafeClaw Gateway");

        // Initialize TEE manager
        if self.config.tee.enabled {
            self.tee_manager.init().await?;
        }

        // Initialize channels
        self.init_channels().await?;

        // Start event processing
        self.start_event_processor().await;

        *self.state.write().await = GatewayState::Running;

        tracing::info!(
            "SafeClaw Gateway started on {}:{}",
            self.config.gateway.host,
            self.config.gateway.port
        );

        Ok(())
    }

    /// Stop the gateway
    pub async fn stop(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if *state != GatewayState::Running {
            return Ok(());
        }
        *state = GatewayState::ShuttingDown;
        drop(state);

        tracing::info!("Stopping SafeClaw Gateway");

        // Stop all channels
        let channels: Vec<Arc<dyn ChannelAdapter>> = {
            let channels = self.channels.read().await;
            channels.values().cloned().collect()
        };

        for channel in channels {
            if let Err(e) = channel.stop().await {
                tracing::warn!("Failed to stop channel {}: {}", channel.name(), e);
            }
        }

        // Shutdown TEE manager
        if self.config.tee.enabled {
            self.tee_manager.shutdown().await?;
        }

        *self.state.write().await = GatewayState::Stopped;

        tracing::info!("SafeClaw Gateway stopped");

        Ok(())
    }

    /// Initialize channel adapters
    async fn init_channels(&self) -> Result<()> {
        let mut channels = self.channels.write().await;

        // Initialize Telegram if configured
        if let Some(telegram_config) = &self.config.channels.telegram {
            let adapter = Arc::new(TelegramAdapter::new(telegram_config.clone()));
            adapter.start(self.event_tx.clone()).await?;
            channels.insert("telegram".to_string(), adapter);
        }

        // Initialize WebChat if configured
        if let Some(webchat_config) = &self.config.channels.webchat {
            if webchat_config.enabled {
                let adapter = Arc::new(WebChatAdapter::new(webchat_config.clone()));
                adapter.start(self.event_tx.clone()).await?;
                channels.insert("webchat".to_string(), adapter);
            }
        }

        // Initialize Feishu if configured
        if let Some(feishu_config) = &self.config.channels.feishu {
            let adapter = Arc::new(FeishuAdapter::new(feishu_config.clone()));
            adapter.start(self.event_tx.clone()).await?;
            channels.insert("feishu".to_string(), adapter);
        }

        // Initialize DingTalk if configured
        if let Some(dingtalk_config) = &self.config.channels.dingtalk {
            let adapter = Arc::new(DingTalkAdapter::new(dingtalk_config.clone()));
            adapter.start(self.event_tx.clone()).await?;
            channels.insert("dingtalk".to_string(), adapter);
        }

        // Initialize WeCom if configured
        if let Some(wecom_config) = &self.config.channels.wecom {
            let adapter = Arc::new(WeComAdapter::new(wecom_config.clone()));
            adapter.start(self.event_tx.clone()).await?;
            channels.insert("wecom".to_string(), adapter);
        }

        // Initialize Slack if configured
        if let Some(slack_config) = &self.config.channels.slack {
            let adapter = Arc::new(SlackAdapter::new(slack_config.clone()));
            adapter.start(self.event_tx.clone()).await?;
            channels.insert("slack".to_string(), adapter);
        }

        // Initialize Discord if configured
        if let Some(discord_config) = &self.config.channels.discord {
            let adapter = Arc::new(DiscordAdapter::new(discord_config.clone()));
            adapter.start(self.event_tx.clone()).await?;
            channels.insert("discord".to_string(), adapter);
        }

        Ok(())
    }

    /// Start the event processor
    async fn start_event_processor(&self) {
        let event_rx = self.event_rx.write().await.take();
        if let Some(mut rx) = event_rx {
            let session_router = self.session_router.clone();
            let tee_manager = self.tee_manager.clone();
            let channels = self.channels.clone();

            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    if let Err(e) =
                        Self::handle_event(event, &session_router, &tee_manager, &channels).await
                    {
                        tracing::error!("Error handling event: {}", e);
                    }
                }
            });
        }
    }

    /// Handle a channel event
    async fn handle_event(
        event: ChannelEvent,
        session_router: &Arc<SessionRouter>,
        tee_manager: &Arc<TeeManager>,
        channels: &Arc<RwLock<HashMap<String, Arc<dyn ChannelAdapter>>>>,
    ) -> Result<()> {
        match event {
            ChannelEvent::Message(message) => {
                tracing::debug!(
                    "Received message from {} on {}: {}",
                    message.sender_id,
                    message.channel,
                    message.content
                );

                // Route the message
                let decision = session_router.route(&message).await?;

                tracing::debug!(
                    "Routing decision: session={}, use_tee={}, level={:?}",
                    decision.session_id,
                    decision.use_tee,
                    decision.classification.level
                );

                // Process the message
                let response = if decision.use_tee {
                    // Process in TEE
                    tee_manager
                        .process_message(&decision.session_id, &message.content)
                        .await?
                } else {
                    // Process locally (placeholder)
                    format!("Received: {}", message.content)
                };

                // Send response
                let channels = channels.read().await;
                if let Some(channel) = channels.get(&message.channel) {
                    let outbound = crate::channels::OutboundMessage::new(
                        &message.channel,
                        &message.chat_id,
                        &response,
                    )
                    .reply_to(&message.channel_message_id);

                    channel.send_message(outbound).await?;
                }
            }
            ChannelEvent::Connected { channel } => {
                tracing::info!("Channel {} connected", channel);
            }
            ChannelEvent::Disconnected { channel, reason } => {
                tracing::warn!("Channel {} disconnected: {}", channel, reason);
            }
            ChannelEvent::Error { channel, error } => {
                tracing::error!("Channel {} error: {}", channel, error);
            }
            _ => {
                tracing::debug!("Unhandled event: {:?}", event);
            }
        }

        Ok(())
    }

    /// Get session manager
    pub fn session_manager(&self) -> &Arc<SessionManager> {
        &self.session_manager
    }

    /// Get TEE manager
    pub fn tee_manager(&self) -> &Arc<TeeManager> {
        &self.tee_manager
    }

    /// Get session router
    pub fn session_router(&self) -> &Arc<SessionRouter> {
        &self.session_router
    }

    /// Get configuration
    pub fn config(&self) -> &SafeClawConfig {
        &self.config
    }

    /// Get active channel names
    pub async fn active_channel_names(&self) -> Vec<String> {
        self.channels.read().await.keys().cloned().collect()
    }

    /// Get channels map
    pub fn channels(&self) -> &Arc<RwLock<HashMap<String, Arc<dyn ChannelAdapter>>>> {
        &self.channels
    }
}

/// Builder for Gateway
pub struct GatewayBuilder {
    config: SafeClawConfig,
}

impl GatewayBuilder {
    /// Create a new builder with default config
    pub fn new() -> Self {
        Self {
            config: SafeClawConfig::default(),
        }
    }

    /// Set the configuration
    pub fn config(mut self, config: SafeClawConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the gateway host
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.config.gateway.host = host.into();
        self
    }

    /// Set the gateway port
    pub fn port(mut self, port: u16) -> Self {
        self.config.gateway.port = port;
        self
    }

    /// Enable/disable TEE
    pub fn tee_enabled(mut self, enabled: bool) -> Self {
        self.config.tee.enabled = enabled;
        self
    }

    /// Build the gateway
    pub fn build(self) -> Gateway {
        Gateway::new(self.config)
    }
}

impl Default for GatewayBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gateway_creation() {
        let gateway = GatewayBuilder::new()
            .host("127.0.0.1")
            .port(18790)
            .tee_enabled(true)
            .build();

        assert_eq!(gateway.state().await, GatewayState::Stopped);
        assert_eq!(gateway.config().gateway.port, 18790);
    }

    #[tokio::test]
    async fn test_gateway_lifecycle() {
        let gateway = GatewayBuilder::new().tee_enabled(false).build();

        gateway.start().await.unwrap();
        assert_eq!(gateway.state().await, GatewayState::Running);

        gateway.stop().await.unwrap();
        assert_eq!(gateway.state().await, GatewayState::Stopped);
    }
}
