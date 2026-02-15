//! Gateway server implementation

use crate::channels::{
    ChannelAdapter, ChannelEvent, DingTalkAdapter, DiscordAdapter, FeishuAdapter, SlackAdapter,
    TelegramAdapter, WeComAdapter, WebChatAdapter,
};
use crate::config::SafeClawConfig;
use crate::error::{Error, Result};
use crate::leakage::{AlertMonitor, AuditEventBus, AuditLog};
use crate::privacy::{Classifier, PolicyEngine};
use crate::session::{SessionManager, SessionRouter};
use serde::Serialize;
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
    session_router: Arc<SessionRouter>,
    channels: Arc<RwLock<HashMap<String, Arc<dyn ChannelAdapter>>>>,
    event_tx: mpsc::Sender<ChannelEvent>,
    event_rx: Arc<RwLock<Option<mpsc::Receiver<ChannelEvent>>>>,
    /// Global audit log shared with the REST API
    global_audit_log: Arc<RwLock<AuditLog>>,
    /// Centralized audit event bus
    audit_bus: Arc<AuditEventBus>,
    /// Alert monitor for rate-based anomaly detection
    alert_monitor: Arc<AlertMonitor>,
}

impl Gateway {
    /// Create a new gateway with the given configuration
    pub fn new(config: SafeClawConfig) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::channel(1000);

        // Create shared global audit log and event bus
        let audit_capacity = config.audit.bus_capacity;
        let global_audit_log = Arc::new(RwLock::new(AuditLog::new(audit_capacity)));
        let audit_bus = Arc::new(AuditEventBus::new(audit_capacity, global_audit_log.clone()));
        let alert_monitor = Arc::new(AlertMonitor::new(config.audit.alert.clone()));

        let session_manager = Arc::new(SessionManager::new(
            config.tee.clone(),
            audit_bus.clone(),
        ));

        let classifier = Arc::new(
            Classifier::new(config.privacy.rules.clone(), config.privacy.default_level)
                .map_err(|e| Error::Privacy(format!("Failed to create classifier: {}", e)))?,
        );
        let policy_engine = Arc::new(PolicyEngine::new());

        let session_router = Arc::new(SessionRouter::new(
            session_manager.clone(),
            classifier,
            policy_engine,
        ));

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(GatewayState::Stopped)),
            session_manager,
            session_router,
            channels: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
            global_audit_log,
            audit_bus,
            alert_monitor,
        })
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

        // Initialize TEE subsystem
        if self.config.tee.enabled {
            self.session_manager.init_tee().await?;
        }

        // Start audit event pipeline: session forwarder + alert monitor
        self.audit_bus
            .spawn_session_forwarder(self.session_manager.isolation().clone());
        if self.config.audit.alert.enabled {
            self.alert_monitor.spawn(self.audit_bus.subscribe());
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

        // Shutdown TEE subsystem
        if self.config.tee.enabled {
            self.session_manager.shutdown_tee().await?;
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
            let session_manager = self.session_manager.clone();
            let channels = self.channels.clone();

            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    if let Err(e) =
                        Self::handle_event(event, &session_router, &session_manager, &channels).await
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
        session_manager: &Arc<SessionManager>,
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
                    // Process in TEE via unified session manager
                    session_manager
                        .process_in_tee(&decision.session_id, &message.content)
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

    /// Get session router
    pub fn session_router(&self) -> &Arc<SessionRouter> {
        &self.session_router
    }

    /// Get configuration
    pub fn config(&self) -> &SafeClawConfig {
        &self.config
    }

    /// Get the global audit log (for constructing AuditState)
    pub fn global_audit_log(&self) -> &Arc<RwLock<AuditLog>> {
        &self.global_audit_log
    }

    /// Get the alert monitor (for constructing AuditState)
    pub fn alert_monitor(&self) -> &Arc<AlertMonitor> {
        &self.alert_monitor
    }

    /// Get the audit event bus
    pub fn audit_bus(&self) -> &Arc<AuditEventBus> {
        &self.audit_bus
    }

    /// Get active channel names
    pub async fn active_channel_names(&self) -> Vec<String> {
        self.channels.read().await.keys().cloned().collect()
    }

    /// Get channels map
    pub fn channels(&self) -> &Arc<RwLock<HashMap<String, Arc<dyn ChannelAdapter>>>> {
        &self.channels
    }

    /// Get event sender for injecting external events (e.g., from a3s-gateway webhooks)
    pub fn event_sender(&self) -> &mpsc::Sender<ChannelEvent> {
        &self.event_tx
    }

    // --- Public API for a3s-gateway integration ---

    /// Process an inbound message and return a response
    ///
    /// This is the main entry point for a3s-gateway to call when it receives
    /// a message routed to SafeClaw. The message is routed through the privacy
    /// classifier and session router, then processed in TEE or locally.
    pub async fn process_message(
        &self,
        message: crate::channels::InboundMessage,
    ) -> Result<ProcessedResponse> {
        // Route the message
        let decision = self.session_router.route(&message).await?;

        tracing::debug!(
            session = %decision.session_id,
            use_tee = decision.use_tee,
            level = ?decision.classification.level,
            "Routing decision"
        );

        // Process the message
        let response_content = if decision.use_tee {
            self.session_manager
                .process_in_tee(&decision.session_id, &message.content)
                .await?
        } else {
            format!("Received: {}", message.content)
        };

        // Build outbound message
        let outbound = crate::channels::OutboundMessage::new(
            &message.channel,
            &message.chat_id,
            &response_content,
        )
        .reply_to(&message.channel_message_id);

        Ok(ProcessedResponse {
            session_id: decision.session_id,
            use_tee: decision.use_tee,
            sensitivity: format!("{:?}", decision.classification.level),
            outbound,
        })
    }

    /// Process a webhook payload from a3s-gateway
    ///
    /// When a3s-gateway receives a webhook from a channel (Telegram, Slack, etc.),
    /// it forwards the raw payload here. SafeClaw parses it using the appropriate
    /// channel adapter and processes the message.
    pub async fn process_webhook(
        &self,
        channel: &str,
        payload: &str,
    ) -> Result<Option<ProcessedResponse>> {
        // Parse the webhook payload into an InboundMessage
        let message = match channel {
            "telegram" | "slack" | "discord" | "feishu" | "dingtalk" | "wecom" => {
                // Create a basic inbound message from webhook payload
                // In production, each channel adapter would parse its specific format
                let parsed: serde_json::Value =
                    serde_json::from_str(payload).map_err(|e| Error::Channel(e.to_string()))?;

                let content = parsed["content"]
                    .as_str()
                    .or_else(|| parsed["text"].as_str())
                    .or_else(|| parsed["message"].as_str())
                    .unwrap_or("")
                    .to_string();

                if content.is_empty() {
                    return Ok(None);
                }

                let sender_id = parsed["sender_id"]
                    .as_str()
                    .or_else(|| parsed["user_id"].as_str())
                    .or_else(|| parsed["from"].as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let chat_id = parsed["chat_id"]
                    .as_str()
                    .or_else(|| parsed["channel_id"].as_str())
                    .unwrap_or(&sender_id)
                    .to_string();

                crate::channels::InboundMessage::new(channel, &sender_id, &chat_id, &content)
            }
            _ => {
                return Err(Error::Channel(format!("Unknown channel: {}", channel)));
            }
        };

        let response = self.process_message(message).await?;
        Ok(Some(response))
    }

    /// Get gateway status information
    pub async fn status(&self) -> GatewayStatus {
        let state = *self.state.read().await;
        let session_count = self.session_manager.session_count().await;
        let channels = self.active_channel_names().await;

        GatewayStatus {
            state: format!("{:?}", state),
            tee_enabled: self.config.tee.enabled,
            session_count,
            channels,
            a3s_gateway_mode: self.config.a3s_gateway.enabled,
        }
    }
}

/// Response from processing a message
#[derive(Debug, Clone, Serialize)]
pub struct ProcessedResponse {
    /// Session ID used for processing
    pub session_id: String,
    /// Whether TEE was used
    pub use_tee: bool,
    /// Sensitivity level detected
    pub sensitivity: String,
    /// Outbound message to send back
    pub outbound: crate::channels::OutboundMessage,
}

/// Gateway status information
#[derive(Debug, Clone, Serialize)]
pub struct GatewayStatus {
    /// Current state
    pub state: String,
    /// Whether TEE is enabled
    pub tee_enabled: bool,
    /// Number of active sessions
    pub session_count: usize,
    /// Active channel names
    pub channels: Vec<String>,
    /// Whether running behind a3s-gateway
    pub a3s_gateway_mode: bool,
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
    pub fn build(self) -> Result<Gateway> {
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
            .build()
            .unwrap();

        assert_eq!(gateway.state().await, GatewayState::Stopped);
        assert_eq!(gateway.config().gateway.port, 18790);
    }

    #[tokio::test]
    async fn test_gateway_lifecycle() {
        let gateway = GatewayBuilder::new().tee_enabled(false).build().unwrap();

        gateway.start().await.unwrap();
        assert_eq!(gateway.state().await, GatewayState::Running);

        gateway.stop().await.unwrap();
        assert_eq!(gateway.state().await, GatewayState::Stopped);
    }
}
