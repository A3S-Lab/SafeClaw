//! WebSocket message bridge between CLI and browser
//!
//! Routes messages between Claude Code CLI (NDJSON) and browser clients (JSON).
//! Maintains per-session state: message history, pending permissions, and message queue.

use crate::agent::session_store::AgentSessionStore;
use crate::agent::types::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};

/// Core message router between CLI and browser WebSocket connections
pub struct AgentBridge {
    /// Per-session internal state
    sessions: Arc<RwLock<HashMap<String, BridgeSession>>>,
    /// Persistence store
    store: Arc<AgentSessionStore>,
    /// Sessions that have already triggered auto-naming
    auto_naming_attempted: Arc<RwLock<HashSet<String>>>,
    /// Callback for first turn completion (session_id, first_user_message)
    first_turn_tx: mpsc::Sender<(String, String)>,
}

/// Internal per-session state
struct BridgeSession {
    id: String,
    /// CLI WebSocket sender (NDJSON strings)
    cli_sender: Option<mpsc::UnboundedSender<String>>,
    /// Browser WebSocket senders (browser_id → JSON strings)
    browser_senders: HashMap<String, mpsc::UnboundedSender<String>>,
    /// Session state (populated from CLI system.init)
    state: AgentSessionState,
    /// Pending permission requests
    pending_permissions: HashMap<String, PermissionRequest>,
    /// Message history for replay on browser reconnect
    message_history: Vec<BrowserIncomingMessage>,
    /// Queued messages when CLI is not connected
    pending_messages: Vec<String>,
    /// CLI's internal session ID (for --resume)
    cli_session_id: Option<String>,
}

impl AgentBridge {
    /// Create a new bridge
    pub fn new(
        store: Arc<AgentSessionStore>,
        first_turn_tx: mpsc::Sender<(String, String)>,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            store,
            auto_naming_attempted: Arc::new(RwLock::new(HashSet::new())),
            first_turn_tx,
        }
    }

    // =========================================================================
    // Connection lifecycle
    // =========================================================================

    /// Handle CLI WebSocket connection open
    pub async fn handle_cli_open(
        &self,
        session_id: &str,
        sender: mpsc::UnboundedSender<String>,
    ) {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .entry(session_id.to_string())
            .or_insert_with(|| BridgeSession {
                id: session_id.to_string(),
                cli_sender: None,
                browser_senders: HashMap::new(),
                state: AgentSessionState::new(session_id.to_string()),
                pending_permissions: HashMap::new(),
                message_history: Vec::new(),
                pending_messages: Vec::new(),
                cli_session_id: None,
            });

        session.cli_sender = Some(sender.clone());

        // Flush pending messages
        let pending: Vec<String> = session.pending_messages.drain(..).collect();
        for msg in pending {
            if sender.send(msg).is_err() {
                tracing::warn!(session_id, "Failed to flush pending message to CLI");
                break;
            }
        }

        // Notify browsers
        self.broadcast_to_browsers_inner(session, &BrowserIncomingMessage::CliConnected);

        tracing::info!(session_id, "CLI connected to bridge");
    }

    /// Handle CLI WebSocket connection close
    pub async fn handle_cli_close(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.cli_sender = None;

            // Cancel all pending permissions
            let cancelled_ids: Vec<String> =
                session.pending_permissions.keys().cloned().collect();
            session.pending_permissions.clear();

            for request_id in cancelled_ids {
                self.broadcast_to_browsers_inner(
                    session,
                    &BrowserIncomingMessage::PermissionCancelled { request_id },
                );
            }

            self.broadcast_to_browsers_inner(
                session,
                &BrowserIncomingMessage::CliDisconnected,
            );

            // Persist
            self.persist_session_inner(session);
        }

        tracing::info!(session_id, "CLI disconnected from bridge");
    }

    /// Handle browser WebSocket connection open
    pub async fn handle_browser_open(
        &self,
        session_id: &str,
        browser_id: &str,
        sender: mpsc::UnboundedSender<String>,
    ) -> bool {
        let sessions = self.sessions.read().await;
        let session = match sessions.get(session_id) {
            Some(s) => s,
            None => return false,
        };

        // Send session_init
        let init_msg = BrowserIncomingMessage::SessionInit {
            session: session.state.clone(),
        };
        Self::send_to_browser(&sender, &init_msg);

        // Send message history
        if !session.message_history.is_empty() {
            let history_msg = BrowserIncomingMessage::MessageHistory {
                messages: session.message_history.clone(),
            };
            Self::send_to_browser(&sender, &history_msg);
        }

        // Send pending permissions
        for perm in session.pending_permissions.values() {
            let perm_msg = BrowserIncomingMessage::PermissionRequest {
                request: perm.clone(),
            };
            Self::send_to_browser(&sender, &perm_msg);
        }

        // Send CLI connection status
        if session.cli_sender.is_none() {
            Self::send_to_browser(&sender, &BrowserIncomingMessage::CliDisconnected);
        }

        drop(sessions);

        // Register browser sender
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session
                .browser_senders
                .insert(browser_id.to_string(), sender);
        }

        tracing::info!(session_id, browser_id, "Browser connected to bridge");
        true
    }

    /// Handle browser WebSocket connection close
    pub async fn handle_browser_close(&self, session_id: &str, browser_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.browser_senders.remove(browser_id);
        }
        tracing::debug!(session_id, browser_id, "Browser disconnected from bridge");
    }

    // =========================================================================
    // CLI → Browser message routing
    // =========================================================================

    /// Route a CLI message to connected browsers
    pub async fn route_cli_message(&self, session_id: &str, msg: CliMessage) {
        let mut sessions = self.sessions.write().await;
        let session = match sessions.get_mut(session_id) {
            Some(s) => s,
            None => {
                tracing::warn!(session_id, "CLI message for unknown session");
                return;
            }
        };

        match msg {
            CliMessage::System(sys) => {
                self.handle_system_message(session, sys);
            }
            CliMessage::Assistant(ast) => {
                let browser_msg = BrowserIncomingMessage::Assistant {
                    message: ast.message,
                    parent_tool_use_id: ast.parent_tool_use_id,
                };
                session.message_history.push(browser_msg.clone());
                self.broadcast_to_browsers_inner(session, &browser_msg);
                self.persist_session_inner(session);
            }
            CliMessage::Result(res) => {
                self.handle_result_message(session, res).await;
            }
            CliMessage::StreamEvent(se) => {
                let browser_msg = BrowserIncomingMessage::StreamEvent {
                    event: se.event,
                    parent_tool_use_id: se.parent_tool_use_id,
                };
                // Stream events are NOT stored in history
                self.broadcast_to_browsers_inner(session, &browser_msg);
            }
            CliMessage::ControlRequest(cr) => {
                if cr.request.subtype == "can_use_tool" {
                    let perm = PermissionRequest {
                        request_id: cr.request_id.clone(),
                        tool_name: cr.request.tool_name.unwrap_or_default(),
                        input: cr.request.input.unwrap_or(serde_json::Value::Null),
                        permission_suggestions: cr.request.permission_suggestions,
                        description: cr.request.description,
                        tool_use_id: cr.request.tool_use_id,
                        agent_id: cr.request.agent_id,
                        timestamp: now_millis(),
                    };
                    session
                        .pending_permissions
                        .insert(cr.request_id.clone(), perm.clone());
                    let browser_msg = BrowserIncomingMessage::PermissionRequest {
                        request: perm,
                    };
                    self.broadcast_to_browsers_inner(session, &browser_msg);
                    self.persist_session_inner(session);
                }
            }
            CliMessage::ToolProgress(tp) => {
                let browser_msg = BrowserIncomingMessage::ToolProgress {
                    tool_use_id: tp.tool_use_id,
                    tool_name: tp.tool_name,
                    elapsed_time_seconds: tp.elapsed_time_seconds,
                };
                self.broadcast_to_browsers_inner(session, &browser_msg);
            }
            CliMessage::ToolUseSummary(tus) => {
                let browser_msg = BrowserIncomingMessage::ToolUseSummary {
                    summary: tus.summary,
                    tool_use_ids: tus.preceding_tool_use_ids,
                };
                self.broadcast_to_browsers_inner(session, &browser_msg);
            }
            CliMessage::AuthStatus(auth) => {
                let browser_msg = BrowserIncomingMessage::AuthStatus {
                    is_authenticating: auth.is_authenticating,
                    output: auth.output,
                    error: auth.error,
                };
                self.broadcast_to_browsers_inner(session, &browser_msg);
            }
            CliMessage::KeepAlive => {
                // Silently consumed
            }
        }
    }

    /// Handle system init/status messages
    fn handle_system_message(&self, session: &mut BridgeSession, sys: CliSystemMessage) {
        match sys.subtype.as_str() {
            "init" => {
                if let Some(cwd) = sys.cwd {
                    session.state.cwd = cwd;
                }
                if let Some(model) = sys.model {
                    session.state.model = model;
                }
                if let Some(tools) = sys.tools {
                    session.state.tools = tools;
                }
                if let Some(pm) = sys.permission_mode {
                    session.state.permission_mode = pm;
                }
                if let Some(ver) = sys.claude_code_version {
                    session.state.claude_code_version = ver;
                }
                if let Some(mcp) = sys.mcp_servers {
                    session.state.mcp_servers = mcp;
                }
                if let Some(agents) = sys.agents {
                    session.state.agents = agents;
                }
                if let Some(cmds) = sys.slash_commands {
                    session.state.slash_commands = cmds;
                }
                if let Some(skills) = sys.skills {
                    session.state.skills = skills;
                }

                // Store CLI session ID for --resume
                if let Some(cli_sid) = sys.session_id {
                    session.cli_session_id = Some(cli_sid);
                }

                let browser_msg = BrowserIncomingMessage::SessionInit {
                    session: session.state.clone(),
                };
                self.broadcast_to_browsers_inner(session, &browser_msg);
                self.persist_session_inner(session);
            }
            "status" => {
                let status = sys.status.as_deref();
                session.state.is_compacting = status == Some("compacting");

                if let Some(pm) = sys.permission_mode {
                    session.state.permission_mode = pm;
                }

                let browser_msg = BrowserIncomingMessage::StatusChange {
                    status: sys.status,
                };
                self.broadcast_to_browsers_inner(session, &browser_msg);
                self.persist_session_inner(session);
            }
            other => {
                tracing::debug!("Unknown system subtype: {}", other);
            }
        }
    }

    /// Handle result messages (turn completion)
    async fn handle_result_message(&self, session: &mut BridgeSession, res: CliResultMessage) {
        // Update session state
        if let Some(cost) = res.total_cost_usd {
            session.state.total_cost_usd = cost;
        }
        if let Some(turns) = res.num_turns {
            session.state.num_turns = turns;
        }
        if let Some(added) = res.total_lines_added {
            session.state.total_lines_added = added;
        }
        if let Some(removed) = res.total_lines_removed {
            session.state.total_lines_removed = removed;
        }
        if let Some(ref usage) = res.model_usage {
            session.state.context_used_percent = compute_context_percent(usage);
        }

        // Store in history
        let browser_msg = BrowserIncomingMessage::Result {
            data: serde_json::to_value(&res).unwrap_or_default(),
        };
        session.message_history.push(browser_msg.clone());
        self.broadcast_to_browsers_inner(session, &browser_msg);

        // Set status to idle
        let status_msg = BrowserIncomingMessage::StatusChange {
            status: Some("idle".to_string()),
        };
        self.broadcast_to_browsers_inner(session, &status_msg);

        self.persist_session_inner(session);

        // Trigger auto-naming on first successful turn
        if !res.is_error {
            let session_id = session.id.clone();
            let should_name = {
                let attempted = self.auto_naming_attempted.read().await;
                !attempted.contains(&session_id)
            };
            if should_name {
                self.auto_naming_attempted
                    .write()
                    .await
                    .insert(session_id.clone());

                // Find first user message
                let first_user_msg = session.message_history.iter().find_map(|msg| {
                    if let BrowserIncomingMessage::UserMessage { content, .. } = msg {
                        Some(content.clone())
                    } else {
                        None
                    }
                });
                if let Some(content) = first_user_msg {
                    let _ = self.first_turn_tx.send((session_id, content)).await;
                }
            }
        }
    }

    // =========================================================================
    // Browser → CLI message routing
    // =========================================================================

    /// Route a browser message to CLI
    pub async fn route_browser_message(&self, session_id: &str, msg: BrowserOutgoingMessage) {
        let mut sessions = self.sessions.write().await;
        let session = match sessions.get_mut(session_id) {
            Some(s) => s,
            None => {
                tracing::warn!(session_id, "Browser message for unknown session");
                return;
            }
        };

        match msg {
            BrowserOutgoingMessage::UserMessage {
                content, images, ..
            } => {
                // Store in history
                let user_msg = BrowserIncomingMessage::UserMessage {
                    content: content.clone(),
                    timestamp: now_millis(),
                };
                session.message_history.push(user_msg);

                // Set status to running
                let status_msg = BrowserIncomingMessage::StatusChange {
                    status: Some("running".to_string()),
                };
                self.broadcast_to_browsers_inner(session, &status_msg);

                // Translate to CLI NDJSON
                let ndjson = user_message_to_cli_ndjson(
                    &content,
                    &images,
                    &session.cli_session_id,
                );
                self.send_to_cli_inner(session, &ndjson);
                self.persist_session_inner(session);
            }
            BrowserOutgoingMessage::PermissionResponse {
                request_id,
                behavior,
                updated_input,
                updated_permissions,
                message,
            } => {
                // Remove from pending
                session.pending_permissions.remove(&request_id);

                let ndjson = if behavior == "allow" {
                    permission_allow_to_cli_ndjson(
                        &request_id,
                        &updated_input,
                        &updated_permissions,
                    )
                } else {
                    permission_deny_to_cli_ndjson(&request_id, &message)
                };
                self.send_to_cli_inner(session, &ndjson);
                self.persist_session_inner(session);
            }
            BrowserOutgoingMessage::Interrupt => {
                let ndjson = interrupt_to_cli_ndjson();
                self.send_to_cli_inner(session, &ndjson);
            }
            BrowserOutgoingMessage::SetModel { model } => {
                let ndjson = set_model_to_cli_ndjson(&model);
                self.send_to_cli_inner(session, &ndjson);
            }
            BrowserOutgoingMessage::SetPermissionMode { mode } => {
                let ndjson = set_permission_mode_to_cli_ndjson(&mode);
                self.send_to_cli_inner(session, &ndjson);
            }
        }
    }

    // =========================================================================
    // Session management
    // =========================================================================

    /// Ensure a bridge session exists
    pub async fn ensure_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions
            .entry(session_id.to_string())
            .or_insert_with(|| BridgeSession {
                id: session_id.to_string(),
                cli_sender: None,
                browser_senders: HashMap::new(),
                state: AgentSessionState::new(session_id.to_string()),
                pending_permissions: HashMap::new(),
                message_history: Vec::new(),
                pending_messages: Vec::new(),
                cli_session_id: None,
            });
    }

    /// Remove a session from the bridge
    pub async fn remove_session(&self, session_id: &str) {
        self.sessions.write().await.remove(session_id);
    }

    /// Check if CLI is connected for a session
    pub async fn is_cli_connected(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .map(|s| s.cli_sender.is_some())
            .unwrap_or(false)
    }

    /// Get the CLI session ID for a session (for --resume)
    pub async fn get_cli_session_id(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .and_then(|s| s.cli_session_id.clone())
    }

    /// Restore sessions from disk
    pub async fn restore_from_disk(&self) {
        let persisted = self.store.load_all();
        let mut sessions = self.sessions.write().await;
        let mut auto_named = self.auto_naming_attempted.write().await;

        for p in persisted {
            // Mark sessions with turns > 0 as already named
            if p.state.num_turns > 0 {
                auto_named.insert(p.id.clone());
            }
            sessions.insert(
                p.id.clone(),
                BridgeSession {
                    id: p.id,
                    cli_sender: None,
                    browser_senders: HashMap::new(),
                    state: p.state,
                    pending_permissions: p.pending_permissions,
                    message_history: p.message_history,
                    pending_messages: p.pending_messages,
                    cli_session_id: None,
                },
            );
        }

        tracing::info!("Restored {} bridge sessions from disk", sessions.len());
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Send a JSON value to CLI as NDJSON
    fn send_to_cli_inner(
        &self,
        session: &mut BridgeSession,
        msg: &serde_json::Value,
    ) {
        let ndjson_line = format!("{}\n", serde_json::to_string(msg).unwrap_or_default());

        if let Some(ref sender) = session.cli_sender {
            if sender.send(ndjson_line).is_err() {
                tracing::warn!(session_id = %session.id, "CLI sender closed");
                session.cli_sender = None;
            }
        } else {
            // Queue for later
            session.pending_messages.push(
                format!("{}\n", serde_json::to_string(msg).unwrap_or_default()),
            );
            tracing::debug!(
                session_id = %session.id,
                "CLI not connected, queued message"
            );
        }
    }

    /// Broadcast a message to all connected browsers for a session
    fn broadcast_to_browsers_inner(
        &self,
        session: &BridgeSession,
        msg: &BrowserIncomingMessage,
    ) {
        Self::broadcast_to_browsers_static(&session.browser_senders, msg);
    }

    /// Static broadcast helper (doesn't need &self)
    fn broadcast_to_browsers_static(
        senders: &HashMap<String, mpsc::UnboundedSender<String>>,
        msg: &BrowserIncomingMessage,
    ) {
        let json = match serde_json::to_string(msg) {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!("Failed to serialize browser message: {}", e);
                return;
            }
        };

        for sender in senders.values() {
            let _ = sender.send(json.clone());
        }
    }

    /// Send a single message to a browser
    fn send_to_browser(
        sender: &mpsc::UnboundedSender<String>,
        msg: &BrowserIncomingMessage,
    ) {
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = sender.send(json);
        }
    }

    /// Persist session state to disk
    fn persist_session_inner(&self, session: &BridgeSession) {
        let persisted = PersistedAgentSession {
            id: session.id.clone(),
            state: session.state.clone(),
            message_history: session.message_history.clone(),
            pending_messages: session.pending_messages.clone(),
            pending_permissions: session.pending_permissions.clone(),
            archived: false,
        };
        // Use store's sync save for now; async save could be used for non-critical paths
        self.store.save_sync(&persisted);
    }
}

/// Current time in milliseconds since UNIX epoch
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_bridge() -> (AgentBridge, TempDir) {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        let store = Arc::new(AgentSessionStore::new(dir.path().to_path_buf()));
        let (tx, _rx) = mpsc::channel(10);
        let bridge = AgentBridge::new(store, tx);
        (bridge, dir)
    }

    #[tokio::test]
    async fn test_ensure_session() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        let sessions = bridge.sessions.read().await;
        assert!(sessions.contains_key("s1"));
    }

    #[tokio::test]
    async fn test_cli_open_and_close() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        let (tx, _rx) = mpsc::unbounded_channel();
        bridge.handle_cli_open("s1", tx).await;

        assert!(bridge.is_cli_connected("s1").await);

        bridge.handle_cli_close("s1").await;
        assert!(!bridge.is_cli_connected("s1").await);
    }

    #[tokio::test]
    async fn test_browser_open_returns_false_for_unknown() {
        let (bridge, _dir) = make_bridge();
        let (tx, _rx) = mpsc::unbounded_channel();
        let result = bridge.handle_browser_open("nonexistent", "b1", tx).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_browser_open_sends_init() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        let (tx, mut rx) = mpsc::unbounded_channel();
        let result = bridge.handle_browser_open("s1", "b1", tx).await;
        assert!(result);

        // Should receive session_init
        let msg = rx.recv().await.unwrap();
        assert!(msg.contains("session_init"));
    }

    #[tokio::test]
    async fn test_route_cli_keep_alive_is_silent() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        // Should not panic or produce output
        bridge.route_cli_message("s1", CliMessage::KeepAlive).await;
    }

    #[tokio::test]
    async fn test_route_cli_system_init() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        let sys = CliSystemMessage {
            subtype: "init".to_string(),
            uuid: None,
            session_id: Some("cli-s1".to_string()),
            cwd: Some("/home/user".to_string()),
            tools: Some(vec!["Bash".to_string(), "Read".to_string()]),
            model: Some("claude-sonnet-4-20250514".to_string()),
            permission_mode: Some("default".to_string()),
            claude_code_version: Some("1.0.0".to_string()),
            mcp_servers: Some(vec![]),
            agents: Some(vec![]),
            slash_commands: Some(vec![]),
            skills: Some(vec![]),
            status: None,
        };

        bridge
            .route_cli_message("s1", CliMessage::System(sys))
            .await;

        let sessions = bridge.sessions.read().await;
        let s = sessions.get("s1").unwrap();
        assert_eq!(s.state.model, "claude-sonnet-4-20250514");
        assert_eq!(s.state.cwd, "/home/user");
        assert_eq!(s.state.tools.len(), 2);
        assert_eq!(s.cli_session_id.as_deref(), Some("cli-s1"));
    }

    #[tokio::test]
    async fn test_route_cli_system_status_compacting() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        let sys = CliSystemMessage {
            subtype: "status".to_string(),
            uuid: None,
            session_id: None,
            cwd: None,
            tools: None,
            model: None,
            permission_mode: None,
            claude_code_version: None,
            mcp_servers: None,
            agents: None,
            slash_commands: None,
            skills: None,
            status: Some("compacting".to_string()),
        };

        bridge
            .route_cli_message("s1", CliMessage::System(sys))
            .await;

        let sessions = bridge.sessions.read().await;
        assert!(sessions.get("s1").unwrap().state.is_compacting);
    }

    #[tokio::test]
    async fn test_route_cli_assistant_stored_in_history() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        let ast = CliAssistantMessage {
            message: AssistantMessageBody {
                id: "msg1".to_string(),
                msg_type: None,
                role: "assistant".to_string(),
                model: "claude-sonnet-4-20250514".to_string(),
                content: vec![ContentBlock::Text {
                    text: "Hello".to_string(),
                }],
                stop_reason: Some("end_turn".to_string()),
                usage: None,
            },
            parent_tool_use_id: None,
            uuid: None,
            session_id: None,
        };

        bridge
            .route_cli_message("s1", CliMessage::Assistant(ast))
            .await;

        let sessions = bridge.sessions.read().await;
        assert_eq!(sessions.get("s1").unwrap().message_history.len(), 1);
    }

    #[tokio::test]
    async fn test_route_cli_control_request_creates_pending_permission() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        let cr = CliControlRequestMessage {
            request_id: "r1".to_string(),
            request: ControlRequestBody {
                subtype: "can_use_tool".to_string(),
                tool_name: Some("Bash".to_string()),
                input: Some(serde_json::json!({"command": "ls"})),
                permission_suggestions: None,
                description: Some("List files".to_string()),
                tool_use_id: Some("tu1".to_string()),
                agent_id: None,
            },
        };

        bridge
            .route_cli_message("s1", CliMessage::ControlRequest(cr))
            .await;

        let sessions = bridge.sessions.read().await;
        let s = sessions.get("s1").unwrap();
        assert_eq!(s.pending_permissions.len(), 1);
        assert!(s.pending_permissions.contains_key("r1"));
    }

    #[tokio::test]
    async fn test_route_browser_user_message() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        // Connect a CLI to receive the message
        let (cli_tx, mut cli_rx) = mpsc::unbounded_channel();
        bridge.handle_cli_open("s1", cli_tx).await;

        let msg = BrowserOutgoingMessage::UserMessage {
            content: "hello".to_string(),
            session_id: None,
            images: None,
        };
        bridge.route_browser_message("s1", msg).await;

        // CLI should receive NDJSON
        let received = cli_rx.recv().await.unwrap();
        assert!(received.contains("\"type\":\"user\""));
        assert!(received.contains("hello"));

        // Message should be in history
        let sessions = bridge.sessions.read().await;
        let s = sessions.get("s1").unwrap();
        assert_eq!(s.message_history.len(), 1);
    }

    #[tokio::test]
    async fn test_route_browser_permission_response_allow() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        // Add a pending permission
        {
            let mut sessions = bridge.sessions.write().await;
            let s = sessions.get_mut("s1").unwrap();
            s.pending_permissions.insert(
                "r1".to_string(),
                PermissionRequest {
                    request_id: "r1".to_string(),
                    tool_name: "Bash".to_string(),
                    input: serde_json::json!({}),
                    permission_suggestions: None,
                    description: None,
                    tool_use_id: None,
                    agent_id: None,
                    timestamp: 0,
                },
            );
        }

        let (cli_tx, mut cli_rx) = mpsc::unbounded_channel();
        bridge.handle_cli_open("s1", cli_tx).await;

        let msg = BrowserOutgoingMessage::PermissionResponse {
            request_id: "r1".to_string(),
            behavior: "allow".to_string(),
            updated_input: None,
            updated_permissions: None,
            message: None,
        };
        bridge.route_browser_message("s1", msg).await;

        // Permission should be removed
        let sessions = bridge.sessions.read().await;
        assert!(sessions.get("s1").unwrap().pending_permissions.is_empty());

        // CLI should receive allow response
        let received = cli_rx.recv().await.unwrap();
        assert!(received.contains("control_response"));
        assert!(received.contains("allow"));
    }

    #[tokio::test]
    async fn test_pending_messages_queue_when_cli_disconnected() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        // Send message without CLI connected
        let msg = BrowserOutgoingMessage::UserMessage {
            content: "queued".to_string(),
            session_id: None,
            images: None,
        };
        bridge.route_browser_message("s1", msg).await;

        // Should be in pending_messages
        let sessions = bridge.sessions.read().await;
        assert_eq!(sessions.get("s1").unwrap().pending_messages.len(), 1);
        drop(sessions);

        // Now connect CLI — pending messages should flush
        let (cli_tx, mut cli_rx) = mpsc::unbounded_channel();
        bridge.handle_cli_open("s1", cli_tx).await;

        let received = cli_rx.recv().await.unwrap();
        assert!(received.contains("queued"));

        // Queue should be empty
        let sessions = bridge.sessions.read().await;
        assert!(sessions.get("s1").unwrap().pending_messages.is_empty());
    }

    #[tokio::test]
    async fn test_cli_close_cancels_permissions() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        // Add permissions
        {
            let mut sessions = bridge.sessions.write().await;
            let s = sessions.get_mut("s1").unwrap();
            s.pending_permissions.insert(
                "r1".to_string(),
                PermissionRequest {
                    request_id: "r1".to_string(),
                    tool_name: "Bash".to_string(),
                    input: serde_json::json!({}),
                    permission_suggestions: None,
                    description: None,
                    tool_use_id: None,
                    agent_id: None,
                    timestamp: 0,
                },
            );
        }

        let (cli_tx, _rx) = mpsc::unbounded_channel();
        bridge.handle_cli_open("s1", cli_tx).await;

        // Connect browser to receive cancellation
        let (browser_tx, mut browser_rx) = mpsc::unbounded_channel();
        bridge.handle_browser_open("s1", "b1", browser_tx).await;
        // Drain the init messages
        while let Ok(msg) = browser_rx.try_recv() {
            if msg.contains("permission_request") {
                // Expected — the pending permission sent on connect
            }
        }

        // Close CLI
        bridge.handle_cli_close("s1").await;

        // Browser should receive permission_cancelled and cli_disconnected
        let mut got_cancelled = false;
        let mut got_disconnected = false;
        while let Ok(msg) = browser_rx.try_recv() {
            if msg.contains("permission_cancelled") {
                got_cancelled = true;
            }
            if msg.contains("cli_disconnected") {
                got_disconnected = true;
            }
        }
        assert!(got_cancelled);
        assert!(got_disconnected);

        // Permissions should be cleared
        let sessions = bridge.sessions.read().await;
        assert!(sessions.get("s1").unwrap().pending_permissions.is_empty());
    }

    #[tokio::test]
    async fn test_remove_session() {
        let (bridge, _dir) = make_bridge();
        bridge.ensure_session("s1").await;

        bridge.remove_session("s1").await;

        let sessions = bridge.sessions.read().await;
        assert!(!sessions.contains_key("s1"));
    }
}
