//! Agent-to-Agent communication via event bus
//!
//! `AgentBus` connects `AgentEngine` to `a3s_event::EventBus`, enabling
//! sessions to send and receive messages across the event bus.
//!
//! ## Subject convention
//!
//! - Broadcast: `events.agent.broadcast.<topic>` — all subscribed sessions receive it
//! - Mention:   `events.agent.mention.<session_id>` — point-to-point
//!
//! ## Execution modes
//!
//! - `auto` (opt-in): incoming message is fed directly into `generate_response`
//! - `confirm` (default): browser receives `AgentMessage` notification; user approves

use crate::agent::engine::AgentEngine;
use crate::agent::types::BrowserIncomingMessage;
use a3s_event::EventBus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// =============================================================================
// Payload type
// =============================================================================

/// Payload carried in agent-to-agent event bus messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessagePayload {
    pub from_session_id: String,
    pub topic: String,
    pub content: String,
}

// =============================================================================
// AgentBus
// =============================================================================

/// Connects `AgentEngine` to the event bus for agent-to-agent messaging.
pub struct AgentBus {
    engine: Arc<AgentEngine>,
    event_bus: Arc<EventBus>,
    /// Per-session auto-execute flag (default: false = confirm mode)
    auto_execute: Arc<RwLock<HashMap<String, bool>>>,
}

impl AgentBus {
    /// Create a new `AgentBus`.
    pub fn new(engine: Arc<AgentEngine>, event_bus: Arc<EventBus>) -> Self {
        Self {
            engine,
            event_bus,
            auto_execute: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set auto-execute mode for a session.
    pub async fn set_auto_execute(&self, session_id: &str, enabled: bool) {
        self.auto_execute
            .write()
            .await
            .insert(session_id.to_string(), enabled);
    }

    /// Get auto-execute mode for a session (default: false).
    pub async fn get_auto_execute(&self, session_id: &str) -> bool {
        *self
            .auto_execute
            .read()
            .await
            .get(session_id)
            .unwrap_or(&false)
    }

    /// Publish a message to another agent via the event bus.
    ///
    /// `target` format:
    /// - `"broadcast:<topic>"` → publishes to `agent.broadcast.<topic>`
    /// - `"mention:<session_id>"` → publishes to `agent.mention.<session_id>`
    pub async fn publish(
        &self,
        from_session_id: &str,
        target: &str,
        content: &str,
    ) -> crate::Result<()> {
        let (category_topic, topic_label) = if let Some(topic) = target.strip_prefix("broadcast:") {
            (format!("broadcast.{}", topic), topic.to_string())
        } else if let Some(sid) = target.strip_prefix("mention:") {
            (format!("mention.{}", sid), sid.to_string())
        } else {
            return Err(crate::Error::Runtime(format!(
                "Invalid agent message target '{}': expected 'broadcast:<topic>' or 'mention:<session_id>'",
                target
            )));
        };

        let payload = AgentMessagePayload {
            from_session_id: from_session_id.to_string(),
            topic: topic_label,
            content: content.to_string(),
        };

        self.event_bus
            .publish(
                "agent",
                &category_topic,
                "agent message",
                from_session_id,
                serde_json::to_value(&payload).unwrap_or_default(),
            )
            .await
            .map_err(|e| {
                crate::Error::Runtime(format!("Failed to publish agent message: {}", e))
            })?;

        Ok(())
    }

    /// Start the subscription loops.
    ///
    /// Spawns two tasks:
    /// 1. Subscribe `agent.broadcast.*` — delivers to all active sessions
    /// 2. Subscribe `agent.mention.*`   — delivers to the specific session
    pub fn start(self: Arc<Self>) {
        let bus = self.clone();
        tokio::spawn(async move {
            bus.run_broadcast_loop().await;
        });

        let bus = self.clone();
        tokio::spawn(async move {
            bus.run_mention_loop().await;
        });
    }

    // =========================================================================
    // Subscription loops
    // =========================================================================

    async fn run_broadcast_loop(&self) {
        // Subscribe to all broadcast messages
        let broadcast_subject = self
            .event_bus
            .provider_arc()
            .build_subject("agent", "broadcast.>");

        let mut sub = match self
            .event_bus
            .provider_arc()
            .subscribe(&broadcast_subject)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("AgentBus: failed to subscribe to broadcast: {}", e);
                return;
            }
        };

        tracing::info!(subject = %broadcast_subject, "AgentBus: broadcast subscription started");

        loop {
            match sub.next().await {
                Ok(Some(received)) => {
                    if let Ok(payload) = serde_json::from_value::<AgentMessagePayload>(
                        received.event.payload.clone(),
                    ) {
                        self.deliver_to_all_sessions(&payload).await;
                    }
                }
                Ok(None) => {
                    tracing::warn!("AgentBus: broadcast subscription closed");
                    break;
                }
                Err(e) => {
                    tracing::warn!("AgentBus: broadcast receive error: {}", e);
                }
            }
        }
    }

    async fn run_mention_loop(&self) {
        // Subscribe to all mention messages (we filter by session_id on delivery)
        let mention_subject = self
            .event_bus
            .provider_arc()
            .build_subject("agent", "mention.>");

        let mut sub = match self
            .event_bus
            .provider_arc()
            .subscribe(&mention_subject)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("AgentBus: failed to subscribe to mentions: {}", e);
                return;
            }
        };

        tracing::info!(subject = %mention_subject, "AgentBus: mention subscription started");

        loop {
            match sub.next().await {
                Ok(Some(received)) => {
                    if let Ok(payload) = serde_json::from_value::<AgentMessagePayload>(
                        received.event.payload.clone(),
                    ) {
                        // Extract target session_id from subject: events.agent.mention.<session_id>
                        let target_session_id = received
                            .event
                            .subject
                            .split('.')
                            .last()
                            .unwrap_or("")
                            .to_string();

                        if !target_session_id.is_empty() {
                            self.deliver_to_session(&target_session_id, &payload).await;
                        }
                    }
                }
                Ok(None) => {
                    tracing::warn!("AgentBus: mention subscription closed");
                    break;
                }
                Err(e) => {
                    tracing::warn!("AgentBus: mention receive error: {}", e);
                }
            }
        }
    }

    // =========================================================================
    // Delivery
    // =========================================================================

    /// Deliver a broadcast message to all active sessions.
    async fn deliver_to_all_sessions(&self, payload: &AgentMessagePayload) {
        let sessions = self.engine.list_sessions().await;
        for session in sessions {
            // Don't deliver to the sender
            if session.session_id == payload.from_session_id {
                continue;
            }
            self.deliver_to_session(&session.session_id, payload).await;
        }
    }

    /// Deliver a message to a specific session.
    async fn deliver_to_session(&self, session_id: &str, payload: &AgentMessagePayload) {
        let auto = self.get_auto_execute(session_id).await;
        let message_id = uuid::Uuid::new_v4().to_string();

        if auto {
            // Auto mode: feed directly into agent generation
            tracing::debug!(
                session_id,
                from = %payload.from_session_id,
                "AgentBus: auto-executing incoming message"
            );
            if let Err(e) = self
                .engine
                .generate_response(session_id, &payload.content)
                .await
            {
                tracing::warn!(session_id, "AgentBus: auto-execute failed: {}", e);
            }
        } else {
            // Confirm mode: notify browser, let user approve
            let msg = BrowserIncomingMessage::AgentMessage {
                message_id,
                from_session_id: payload.from_session_id.clone(),
                topic: payload.topic.clone(),
                content: payload.content.clone(),
                auto_execute: false,
            };
            self.engine.broadcast_to_session(session_id, &msg).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::session_store::AgentSessionStore;
    use a3s_code::config::CodeConfig;
    use a3s_event::MemoryProvider;
    use tempfile::TempDir;

    async fn make_engine(dir: &std::path::Path) -> Arc<AgentEngine> {
        let code_config = CodeConfig {
            sessions_dir: Some(dir.to_path_buf()),
            ..Default::default()
        };
        let cwd = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
            .to_string_lossy()
            .to_string();
        let tool_executor = Arc::new(a3s_code::tools::ToolExecutor::new(cwd));
        let session_manager = Arc::new(
            a3s_code::session::SessionManager::with_persistence(None, tool_executor, dir)
                .await
                .unwrap(),
        );
        let store = Arc::new(AgentSessionStore::new(dir.join("ui-state")));
        Arc::new(
            AgentEngine::new(session_manager, code_config, store)
                .await
                .unwrap(),
        )
    }

    #[tokio::test]
    async fn test_publish_broadcast_invalid_target() {
        let dir = TempDir::new().unwrap();
        let engine = make_engine(dir.path()).await;
        let event_bus = Arc::new(EventBus::new(MemoryProvider::default()));
        let bus = Arc::new(AgentBus::new(engine, event_bus));

        let result = bus.publish("session-1", "invalid-target", "hello").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid agent message target"));
    }

    #[tokio::test]
    async fn test_publish_broadcast_valid() {
        let dir = TempDir::new().unwrap();
        let engine = make_engine(dir.path()).await;
        let event_bus = Arc::new(EventBus::new(MemoryProvider::default()));
        let bus = Arc::new(AgentBus::new(engine, event_bus.clone()));

        let result = bus
            .publish("session-1", "broadcast:code-review", "please review")
            .await;
        assert!(result.is_ok());

        // Verify event was published
        let counts = event_bus.counts(10).await.unwrap();
        assert_eq!(counts.total, 1);
    }

    #[tokio::test]
    async fn test_publish_mention_valid() {
        let dir = TempDir::new().unwrap();
        let engine = make_engine(dir.path()).await;
        let event_bus = Arc::new(EventBus::new(MemoryProvider::default()));
        let bus = Arc::new(AgentBus::new(engine, event_bus.clone()));

        let result = bus
            .publish("session-1", "mention:session-2", "hey you")
            .await;
        assert!(result.is_ok());

        let counts = event_bus.counts(10).await.unwrap();
        assert_eq!(counts.total, 1);
    }

    #[tokio::test]
    async fn test_auto_execute_default_false() {
        let dir = TempDir::new().unwrap();
        let engine = make_engine(dir.path()).await;
        let event_bus = Arc::new(EventBus::new(MemoryProvider::default()));
        let bus = AgentBus::new(engine, event_bus);

        assert!(!bus.get_auto_execute("any-session").await);
    }

    #[tokio::test]
    async fn test_auto_execute_toggle() {
        let dir = TempDir::new().unwrap();
        let engine = make_engine(dir.path()).await;
        let event_bus = Arc::new(EventBus::new(MemoryProvider::default()));
        let bus = AgentBus::new(engine, event_bus);

        bus.set_auto_execute("s1", true).await;
        assert!(bus.get_auto_execute("s1").await);

        bus.set_auto_execute("s1", false).await;
        assert!(!bus.get_auto_execute("s1").await);
    }
}
