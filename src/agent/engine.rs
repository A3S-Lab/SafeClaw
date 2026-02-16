//! Agent engine — direct a3s-code library integration
//!
//! Wraps `a3s_code::session::SessionManager` to provide in-process agent
//! execution, replacing the previous CLI subprocess architecture.
//!
//! ```text
//! UI <-WS(JSON)-> handler.rs -> engine.rs -> a3s-code SessionManager (in-process)
//!                                 └── session_store.rs (UI state only)
//! ```

use crate::agent::session_store::AgentSessionStore;
use crate::agent::types::*;
use a3s_code::agent::AgentEvent;
use a3s_code::config::CodeConfig;
use a3s_code::session::SessionManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Core engine wrapping a3s-code's `SessionManager`.
///
/// Manages per-session UI state (browser senders, message history,
/// pending permissions) alongside the a3s-code session lifecycle.
pub struct AgentEngine {
    session_manager: Arc<SessionManager>,
    code_config: CodeConfig,
    sessions: Arc<RwLock<HashMap<String, EngineSession>>>,
    store: Arc<AgentSessionStore>,
}

/// Per-session UI state tracked by the engine.
struct EngineSession {
    id: String,
    browser_senders: HashMap<String, mpsc::UnboundedSender<String>>,
    state: AgentSessionState,
    message_history: Vec<BrowserIncomingMessage>,
    pending_permissions: HashMap<String, PermissionRequest>,
    generation_handle: Option<tokio::task::JoinHandle<()>>,
    name: Option<String>,
    archived: bool,
    created_at: u64,
    cwd: String,
    model: Option<String>,
    permission_mode: Option<String>,
}

impl AgentEngine {
    /// Create a new engine from a pre-built `SessionManager` and config.
    pub async fn new(
        session_manager: Arc<SessionManager>,
        code_config: CodeConfig,
        store: Arc<AgentSessionStore>,
    ) -> crate::Result<Self> {
        let engine = Self {
            session_manager,
            code_config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            store,
        };

        // Restore persisted UI state from disk
        engine.restore_from_disk().await;

        Ok(engine)
    }

    // =========================================================================
    // Session CRUD
    // =========================================================================

    /// Create a new agent session.
    ///
    /// Creates both an a3s-code session and the corresponding UI state.
    pub async fn create_session(
        &self,
        session_id: &str,
        model: Option<String>,
        permission_mode: Option<String>,
        cwd: Option<String>,
    ) -> crate::Result<AgentProcessInfo> {
        let workspace = cwd.unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
                .to_string_lossy()
                .to_string()
        });

        // Build a3s-code session config
        let mut session_config = a3s_code::session::SessionConfig {
            name: String::new(),
            workspace: workspace.clone(),
            ..Default::default()
        };

        // Set permission policy based on mode
        if let Some(ref mode) = permission_mode {
            session_config.permission_policy = Some(permission_mode_to_policy(mode));
        }

        // Create a3s-code session
        self.session_manager
            .create_session(session_id.to_string(), session_config)
            .await
            .map_err(|e| {
                crate::Error::Gateway(format!("Failed to create a3s-code session: {}", e))
            })?;

        // Configure model-specific LLM client if requested
        if let Some(ref model_id) = model {
            if let Err(e) = self.configure_model_for_session(session_id, model_id).await {
                tracing::warn!(
                    session_id = %session_id,
                    model = %model_id,
                    "Failed to configure model: {}",
                    e
                );
            }
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let engine_session = EngineSession {
            id: session_id.to_string(),
            browser_senders: HashMap::new(),
            state: AgentSessionState::new(session_id.to_string()),
            message_history: Vec::new(),
            pending_permissions: HashMap::new(),
            generation_handle: None,
            name: None,
            archived: false,
            created_at: now,
            cwd: workspace.clone(),
            model: model.clone(),
            permission_mode: permission_mode.clone(),
        };

        // Update state fields
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.to_string(), engine_session);

        // Update the session state with model/cwd info
        if let Some(es) = sessions.get_mut(session_id) {
            es.state.model = model.clone().unwrap_or_default();
            es.state.cwd = workspace.clone();
            es.state.permission_mode = permission_mode
                .clone()
                .unwrap_or_else(|| "default".to_string());
            // Populate tool names from executor
            es.state.tools = self
                .session_manager
                .list_tools()
                .iter()
                .map(|t| t.name.clone())
                .collect();
        }

        let info = AgentProcessInfo {
            session_id: session_id.to_string(),
            pid: None,
            state: AgentProcessState::Connected,
            exit_code: None,
            model,
            permission_mode,
            cwd: workspace,
            created_at: now,
            cli_session_id: None,
            archived: false,
            name: None,
        };

        Ok(info)
    }

    /// Destroy a session and clean up all state.
    pub async fn destroy_session(&self, session_id: &str) -> crate::Result<()> {
        // Cancel any running generation
        {
            let mut sessions = self.sessions.write().await;
            if let Some(es) = sessions.get_mut(session_id) {
                if let Some(handle) = es.generation_handle.take() {
                    handle.abort();
                }
            }
        }

        // Destroy a3s-code session (ignore error if it doesn't exist there)
        let _ = self.session_manager.destroy_session(session_id).await;

        // Remove UI state
        self.sessions.write().await.remove(session_id);

        // Remove from disk
        self.store.remove(session_id).await;

        Ok(())
    }

    /// List all sessions as `AgentProcessInfo`.
    pub async fn list_sessions(&self) -> Vec<AgentProcessInfo> {
        let sessions = self.sessions.read().await;
        sessions.values().map(|es| es.to_process_info()).collect()
    }

    /// Get a single session's info.
    pub async fn get_session(&self, session_id: &str) -> Option<AgentProcessInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|es| es.to_process_info())
    }

    /// Set a session's display name.
    pub async fn set_name(&self, session_id: &str, name: String) {
        let mut sessions = self.sessions.write().await;
        if let Some(es) = sessions.get_mut(session_id) {
            es.name = Some(name.clone());

            // Notify connected browsers
            let msg = BrowserIncomingMessage::SessionNameUpdate { name };
            let json = serde_json::to_string(&msg).unwrap_or_default();
            for sender in es.browser_senders.values() {
                let _ = sender.send(json.clone());
            }

            self.persist_session(es);
        }
    }

    /// Set a session's archived flag.
    pub async fn set_archived(&self, session_id: &str, archived: bool) {
        let mut sessions = self.sessions.write().await;
        if let Some(es) = sessions.get_mut(session_id) {
            es.archived = archived;
            self.persist_session(es);
        }
    }

    // =========================================================================
    // Browser WebSocket lifecycle
    // =========================================================================

    /// Register a browser WebSocket connection.
    ///
    /// Sends `SessionInit`, `MessageHistory`, and pending permissions for
    /// state replay on reconnect. Returns `false` if the session doesn't exist.
    pub async fn handle_browser_open(
        &self,
        session_id: &str,
        browser_id: &str,
        sender: mpsc::UnboundedSender<String>,
    ) -> bool {
        let mut sessions = self.sessions.write().await;
        let es = match sessions.get_mut(session_id) {
            Some(es) => es,
            None => return false,
        };

        // Send session_init
        let init_msg = BrowserIncomingMessage::SessionInit {
            session: es.state.clone(),
        };
        if let Ok(json) = serde_json::to_string(&init_msg) {
            let _ = sender.send(json);
        }

        // Send message history
        if !es.message_history.is_empty() {
            let history_msg = BrowserIncomingMessage::MessageHistory {
                messages: es.message_history.clone(),
            };
            if let Ok(json) = serde_json::to_string(&history_msg) {
                let _ = sender.send(json);
            }
        }

        // Send pending permission requests
        for perm in es.pending_permissions.values() {
            let perm_msg = BrowserIncomingMessage::PermissionRequest {
                request: perm.clone(),
            };
            if let Ok(json) = serde_json::to_string(&perm_msg) {
                let _ = sender.send(json);
            }
        }

        es.browser_senders.insert(browser_id.to_string(), sender);
        true
    }

    /// Unregister a browser WebSocket connection.
    pub async fn handle_browser_close(&self, session_id: &str, browser_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(es) = sessions.get_mut(session_id) {
            es.browser_senders.remove(browser_id);
        }
    }

    // =========================================================================
    // Browser message dispatch
    // =========================================================================

    /// Handle a message from a browser client.
    pub async fn handle_browser_message(&self, session_id: &str, msg: BrowserOutgoingMessage) {
        match msg {
            BrowserOutgoingMessage::UserMessage {
                content, images: _, ..
            } => {
                // Store in history
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let user_msg = BrowserIncomingMessage::UserMessage {
                    content: content.clone(),
                    timestamp: now,
                };

                {
                    let mut sessions = self.sessions.write().await;
                    if let Some(es) = sessions.get_mut(session_id) {
                        es.message_history.push(user_msg.clone());

                        // Broadcast user message to all browsers for echo
                        let json = serde_json::to_string(&user_msg).unwrap_or_default();
                        for sender in es.browser_senders.values() {
                            let _ = sender.send(json.clone());
                        }
                    }
                }

                // Start generation
                self.spawn_generation(session_id, &content).await;
            }
            BrowserOutgoingMessage::PermissionResponse {
                request_id,
                behavior,
                ..
            } => {
                let approved = behavior == "allow";

                // Remove from pending
                {
                    let mut sessions = self.sessions.write().await;
                    if let Some(es) = sessions.get_mut(session_id) {
                        es.pending_permissions.remove(&request_id);
                    }
                }

                if let Err(e) = self
                    .session_manager
                    .confirm_tool(session_id, &request_id, approved, None)
                    .await
                {
                    tracing::warn!(
                        session_id = %session_id,
                        request_id = %request_id,
                        "Failed to confirm tool: {}",
                        e
                    );
                }
            }
            BrowserOutgoingMessage::Interrupt => {
                // Cancel running generation
                {
                    let mut sessions = self.sessions.write().await;
                    if let Some(es) = sessions.get_mut(session_id) {
                        if let Some(handle) = es.generation_handle.take() {
                            handle.abort();
                        }
                    }
                }
                let _ = self.session_manager.cancel_operation(session_id).await;

                // Notify browsers of idle state
                self.broadcast(
                    session_id,
                    &BrowserIncomingMessage::StatusChange {
                        status: Some("idle".to_string()),
                    },
                )
                .await;
            }
            BrowserOutgoingMessage::SetModel { model } => {
                if let Err(e) = self.configure_model_for_session(session_id, &model).await {
                    tracing::warn!(
                        session_id = %session_id,
                        model = %model,
                        "Failed to set model: {}",
                        e
                    );
                } else {
                    let mut sessions = self.sessions.write().await;
                    if let Some(es) = sessions.get_mut(session_id) {
                        es.model = Some(model.clone());
                        es.state.model = model;
                    }
                }
            }
            BrowserOutgoingMessage::SetPermissionMode { mode } => {
                let policy = permission_mode_to_policy(&mode);
                if let Err(e) = self
                    .session_manager
                    .set_permission_policy(session_id, policy)
                    .await
                {
                    tracing::warn!(
                        session_id = %session_id,
                        mode = %mode,
                        "Failed to set permission mode: {}",
                        e
                    );
                } else {
                    let mut sessions = self.sessions.write().await;
                    if let Some(es) = sessions.get_mut(session_id) {
                        es.permission_mode = Some(mode.clone());
                        es.state.permission_mode = mode;
                    }
                }
            }
        }
    }

    // =========================================================================
    // Generation
    // =========================================================================

    /// Spawn a streaming generation task for the given session.
    async fn spawn_generation(&self, session_id: &str, prompt: &str) {
        // Cancel any existing generation
        {
            let mut sessions = self.sessions.write().await;
            if let Some(es) = sessions.get_mut(session_id) {
                if let Some(handle) = es.generation_handle.take() {
                    handle.abort();
                }
            }
        }

        // Notify browsers that we're running
        self.broadcast(
            session_id,
            &BrowserIncomingMessage::StatusChange {
                status: Some("running".to_string()),
            },
        )
        .await;

        // Start streaming generation
        let result = self
            .session_manager
            .generate_streaming(session_id, prompt)
            .await;

        let (mut event_rx, _join_handle) = match result {
            Ok((rx, jh)) => (rx, jh),
            Err(e) => {
                tracing::error!(
                    session_id = %session_id,
                    "Failed to start generation: {}",
                    e
                );
                self.broadcast(
                    session_id,
                    &BrowserIncomingMessage::Error {
                        message: format!("Failed to start generation: {}", e),
                    },
                )
                .await;
                self.broadcast(
                    session_id,
                    &BrowserIncomingMessage::StatusChange {
                        status: Some("idle".to_string()),
                    },
                )
                .await;
                return;
            }
        };

        let sessions = self.sessions.clone();
        let store = self.store.clone();
        let sid = session_id.to_string();

        let handle = tokio::spawn(async move {
            let mut text_buffer = String::new();

            while let Some(event) = event_rx.recv().await {
                // Accumulate text for final message
                if let AgentEvent::TextDelta { ref text } = event {
                    text_buffer.push_str(text);
                }

                let browser_messages = translate_event(&event);

                // Store permissions and update session state
                match &event {
                    AgentEvent::ConfirmationRequired {
                        tool_id,
                        tool_name,
                        args,
                        ..
                    } => {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        let perm = PermissionRequest {
                            request_id: tool_id.clone(),
                            tool_name: tool_name.clone(),
                            input: args.clone(),
                            permission_suggestions: None,
                            description: None,
                            tool_use_id: Some(tool_id.clone()),
                            agent_id: None,
                            timestamp: now,
                        };
                        let mut sessions = sessions.write().await;
                        if let Some(es) = sessions.get_mut(&sid) {
                            es.pending_permissions.insert(tool_id.clone(), perm);
                        }
                    }
                    AgentEvent::ConfirmationReceived { tool_id, .. }
                    | AgentEvent::ConfirmationTimeout { tool_id, .. } => {
                        let mut sessions = sessions.write().await;
                        if let Some(es) = sessions.get_mut(&sid) {
                            es.pending_permissions.remove(tool_id);
                        }
                    }
                    AgentEvent::TurnEnd { turn, usage } => {
                        let mut sessions = sessions.write().await;
                        if let Some(es) = sessions.get_mut(&sid) {
                            es.state.num_turns = *turn as u32;
                            es.state.total_cost_usd += estimate_cost_from_usage(usage);
                        }
                    }
                    AgentEvent::End { ref text, .. } => {
                        // Build complete assistant message and store in history
                        let assistant_msg = BrowserIncomingMessage::Assistant {
                            message: AssistantMessageBody {
                                id: uuid::Uuid::new_v4().to_string(),
                                msg_type: Some("message".to_string()),
                                role: "assistant".to_string(),
                                model: String::new(),
                                content: vec![ContentBlock::Text {
                                    text: if text.is_empty() {
                                        text_buffer.clone()
                                    } else {
                                        text.clone()
                                    },
                                }],
                                stop_reason: Some("end_turn".to_string()),
                                usage: None,
                            },
                            parent_tool_use_id: None,
                        };

                        let mut sessions = sessions.write().await;
                        if let Some(es) = sessions.get_mut(&sid) {
                            es.message_history.push(assistant_msg);
                            persist_session_with_store(es, &store);
                        }
                    }
                    _ => {}
                }

                // Broadcast translated messages to all browsers
                let sessions_read = sessions.read().await;
                if let Some(es) = sessions_read.get(&sid) {
                    for browser_msg in &browser_messages {
                        if let Ok(json) = serde_json::to_string(browser_msg) {
                            for sender in es.browser_senders.values() {
                                let _ = sender.send(json.clone());
                            }
                        }
                    }
                }
            }

            // Generation complete — send idle status
            let idle = BrowserIncomingMessage::StatusChange {
                status: Some("idle".to_string()),
            };
            let sessions_read = sessions.read().await;
            if let Some(es) = sessions_read.get(&sid) {
                if let Ok(json) = serde_json::to_string(&idle) {
                    for sender in es.browser_senders.values() {
                        let _ = sender.send(json.clone());
                    }
                }
            }
            drop(sessions_read);

            // Clear generation handle
            let mut sessions = sessions.write().await;
            if let Some(es) = sessions.get_mut(&sid) {
                es.generation_handle = None;
            }
        });

        // Store the handle
        let mut sessions = self.sessions.write().await;
        if let Some(es) = sessions.get_mut(session_id) {
            es.generation_handle = Some(handle);
        }
    }

    // =========================================================================
    // Channel message processing (non-WebSocket)
    // =========================================================================

    /// Generate a text response for a channel message.
    ///
    /// Unlike `spawn_generation` (browser WebSocket), this method collects all
    /// streaming events and returns the final text. Used by the Gateway's event
    /// processor to handle messages from Telegram, Slack, Discord, etc.
    ///
    /// If no agent session exists for the given ID, one is created with
    /// `trust` permission mode (auto-approve all tool calls for chat channels).
    pub async fn generate_response(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> crate::Result<String> {
        // Ensure agent session exists
        if self.get_session(session_id).await.is_none() {
            self.create_session(session_id, None, Some("trust".to_string()), None)
                .await?;
        }

        // Start streaming generation
        let (mut event_rx, _join_handle) = self
            .session_manager
            .generate_streaming(session_id, prompt)
            .await
            .map_err(|e| {
                crate::Error::Gateway(format!("Failed to start generation: {}", e))
            })?;

        // Collect text from streaming events
        let mut text = String::new();
        while let Some(event) = event_rx.recv().await {
            match &event {
                AgentEvent::TextDelta { text: delta } => {
                    text.push_str(delta);
                }
                AgentEvent::End {
                    text: final_text, ..
                } => {
                    if !final_text.is_empty() {
                        return Ok(final_text.clone());
                    }
                }
                AgentEvent::Error { message } => {
                    return Err(crate::Error::Gateway(format!(
                        "Agent error: {}",
                        message
                    )));
                }
                _ => {}
            }
        }

        if text.is_empty() {
            Ok("I received your message but couldn't generate a response.".to_string())
        } else {
            Ok(text)
        }
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    /// Broadcast a message to all browser connections for a session.
    async fn broadcast(&self, session_id: &str, msg: &BrowserIncomingMessage) {
        let sessions = self.sessions.read().await;
        if let Some(es) = sessions.get(session_id) {
            if let Ok(json) = serde_json::to_string(msg) {
                for sender in es.browser_senders.values() {
                    let _ = sender.send(json.clone());
                }
            }
        }
    }

    /// Configure the LLM client for a session based on model ID.
    ///
    /// Searches all providers for the given model ID and constructs the
    /// appropriate `LlmConfig`.
    async fn configure_model_for_session(
        &self,
        session_id: &str,
        model_id: &str,
    ) -> crate::Result<()> {
        // Search across all providers for this model
        let llm_config = self
            .code_config
            .providers
            .iter()
            .find_map(|p| self.code_config.llm_config(&p.name, model_id))
            .ok_or_else(|| {
                crate::Error::Gateway(format!(
                    "No LLM config found for model '{}' in code config",
                    model_id
                ))
            })?;

        self.session_manager
            .configure(session_id, None, None, Some(llm_config))
            .await
            .map_err(|e| crate::Error::Gateway(format!("Failed to configure session: {}", e)))
    }

    /// Persist an engine session to disk via the store.
    fn persist_session(&self, es: &EngineSession) {
        let persisted = PersistedAgentSession {
            id: es.id.clone(),
            state: es.state.clone(),
            message_history: es.message_history.clone(),
            pending_messages: Vec::new(),
            pending_permissions: es.pending_permissions.clone(),
            archived: es.archived,
        };
        self.store.save_sync(&persisted);
    }

    /// Restore sessions from disk on startup.
    async fn restore_from_disk(&self) {
        let persisted_sessions = self.store.load_all();
        let mut sessions = self.sessions.write().await;

        for ps in persisted_sessions {
            let es = EngineSession {
                id: ps.id.clone(),
                browser_senders: HashMap::new(),
                state: ps.state.clone(),
                message_history: ps.message_history,
                pending_permissions: ps.pending_permissions,
                generation_handle: None,
                name: None,
                archived: ps.archived,
                created_at: 0,
                cwd: ps.state.cwd.clone(),
                model: if ps.state.model.is_empty() {
                    None
                } else {
                    Some(ps.state.model.clone())
                },
                permission_mode: Some(ps.state.permission_mode.clone()),
            };

            // Re-create a3s-code session if it doesn't exist already
            let session_config = a3s_code::session::SessionConfig {
                name: String::new(),
                workspace: es.cwd.clone(),
                ..Default::default()
            };
            let _ = self
                .session_manager
                .create_session(ps.id.clone(), session_config)
                .await;

            sessions.insert(ps.id, es);
        }

        tracing::info!(count = sessions.len(), "Restored agent sessions from disk");
    }
}

/// Persist session helper that can be called from spawned tasks.
fn persist_session_with_store(es: &EngineSession, store: &AgentSessionStore) {
    let persisted = PersistedAgentSession {
        id: es.id.clone(),
        state: es.state.clone(),
        message_history: es.message_history.clone(),
        pending_messages: Vec::new(),
        pending_permissions: es.pending_permissions.clone(),
        archived: es.archived,
    };
    store.save_sync(&persisted);
}

impl EngineSession {
    /// Convert to the REST API response type.
    fn to_process_info(&self) -> AgentProcessInfo {
        AgentProcessInfo {
            session_id: self.id.clone(),
            pid: None,
            state: if self.generation_handle.is_some() {
                AgentProcessState::Running
            } else {
                AgentProcessState::Connected
            },
            exit_code: None,
            model: self.model.clone(),
            permission_mode: self.permission_mode.clone(),
            cwd: self.cwd.clone(),
            created_at: self.created_at,
            cli_session_id: None,
            archived: self.archived,
            name: self.name.clone(),
        }
    }
}

// =============================================================================
// Event translation (pure functions)
// =============================================================================

/// Translate an `AgentEvent` into zero or more `BrowserIncomingMessage`s.
///
/// This is a pure function with no side effects, making it easy to test.
pub fn translate_event(event: &AgentEvent) -> Vec<BrowserIncomingMessage> {
    match event {
        AgentEvent::Start { .. } => {
            vec![BrowserIncomingMessage::StatusChange {
                status: Some("running".to_string()),
            }]
        }
        AgentEvent::TextDelta { text } => {
            vec![BrowserIncomingMessage::StreamEvent {
                event: serde_json::json!({
                    "type": "content_block_delta",
                    "delta": {
                        "type": "text_delta",
                        "text": text,
                    }
                }),
                parent_tool_use_id: None,
            }]
        }
        AgentEvent::ToolStart { id, name } => {
            vec![BrowserIncomingMessage::StreamEvent {
                event: serde_json::json!({
                    "type": "content_block_start",
                    "content_block": {
                        "type": "tool_use",
                        "id": id,
                        "name": name,
                    }
                }),
                parent_tool_use_id: None,
            }]
        }
        AgentEvent::ToolEnd { id, name, .. } => {
            vec![BrowserIncomingMessage::ToolProgress {
                tool_use_id: id.clone(),
                tool_name: name.clone(),
                elapsed_time_seconds: 0.0,
            }]
        }
        AgentEvent::TurnEnd { turn, usage } => {
            vec![BrowserIncomingMessage::SessionUpdate {
                session: serde_json::json!({
                    "num_turns": turn,
                    "total_cost_usd": estimate_cost_from_usage(usage),
                    "context_used_percent": 0.0,
                }),
            }]
        }
        AgentEvent::End { text, usage } => {
            let result_data = serde_json::json!({
                "subtype": "success",
                "is_error": false,
                "result": text,
                "total_cost_usd": estimate_cost_from_usage(usage),
            });
            vec![
                BrowserIncomingMessage::Result { data: result_data },
                BrowserIncomingMessage::StatusChange {
                    status: Some("idle".to_string()),
                },
            ]
        }
        AgentEvent::Error { message } => {
            vec![BrowserIncomingMessage::Error {
                message: message.clone(),
            }]
        }
        AgentEvent::ConfirmationRequired {
            tool_id,
            tool_name,
            args,
            ..
        } => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            vec![BrowserIncomingMessage::PermissionRequest {
                request: PermissionRequest {
                    request_id: tool_id.clone(),
                    tool_name: tool_name.clone(),
                    input: args.clone(),
                    permission_suggestions: None,
                    description: None,
                    tool_use_id: Some(tool_id.clone()),
                    agent_id: None,
                    timestamp: now,
                },
            }]
        }
        AgentEvent::ConfirmationReceived { tool_id, .. }
        | AgentEvent::ConfirmationTimeout { tool_id, .. } => {
            vec![BrowserIncomingMessage::PermissionCancelled {
                request_id: tool_id.clone(),
            }]
        }
        AgentEvent::PermissionDenied { reason, .. } => {
            vec![BrowserIncomingMessage::Error {
                message: format!("Permission denied: {}", reason),
            }]
        }
        AgentEvent::SubagentStart { .. }
        | AgentEvent::SubagentProgress { .. }
        | AgentEvent::SubagentEnd { .. } => {
            // Forward subagent events as generic stream events
            if let Ok(val) = serde_json::to_value(event) {
                vec![BrowserIncomingMessage::StreamEvent {
                    event: val,
                    parent_tool_use_id: None,
                }]
            } else {
                vec![]
            }
        }
        AgentEvent::TodoUpdated { .. } => {
            if let Ok(val) = serde_json::to_value(event) {
                vec![BrowserIncomingMessage::StreamEvent {
                    event: val,
                    parent_tool_use_id: None,
                }]
            } else {
                vec![]
            }
        }
        AgentEvent::ContextCompacted { .. } => {
            vec![BrowserIncomingMessage::SessionUpdate {
                session: serde_json::json!({
                    "is_compacting": false,
                }),
            }]
        }
        AgentEvent::ToolOutputDelta { id, name, delta } => {
            vec![BrowserIncomingMessage::StreamEvent {
                event: serde_json::json!({
                    "type": "tool_output_delta",
                    "tool_use_id": id,
                    "tool_name": name,
                    "delta": delta,
                }),
                parent_tool_use_id: Some(id.clone()),
            }]
        }
        // Suppress internal events that don't need to reach the browser
        AgentEvent::TurnStart { .. }
        | AgentEvent::ContextResolving { .. }
        | AgentEvent::ContextResolved { .. }
        | AgentEvent::CommandDeadLettered { .. }
        | AgentEvent::CommandRetry { .. }
        | AgentEvent::QueueAlert { .. }
        | AgentEvent::MemoryStored { .. }
        | AgentEvent::MemoryRecalled { .. }
        | AgentEvent::MemoriesSearched { .. }
        | AgentEvent::MemoryCleared { .. }
        | AgentEvent::PlanningStart { .. }
        | AgentEvent::PlanningEnd { .. }
        | AgentEvent::StepStart { .. }
        | AgentEvent::StepEnd { .. }
        | AgentEvent::GoalExtracted { .. }
        | AgentEvent::GoalProgress { .. }
        | AgentEvent::GoalAchieved { .. }
        | AgentEvent::ExternalTaskPending { .. }
        | AgentEvent::ExternalTaskCompleted { .. }
        | AgentEvent::PersistenceFailed { .. } => {
            vec![]
        }
    }
}

/// Convert a permission mode string to a `PermissionPolicy`.
fn permission_mode_to_policy(mode: &str) -> a3s_code::permissions::PermissionPolicy {
    match mode {
        "plan" | "strict" => a3s_code::permissions::PermissionPolicy::strict(),
        "yolo" | "permissive" | "trust" => a3s_code::permissions::PermissionPolicy::permissive(),
        _ => a3s_code::permissions::PermissionPolicy::new(),
    }
}

/// Rough cost estimate from `TokenUsage`.
fn estimate_cost_from_usage(usage: &a3s_code::llm::TokenUsage) -> f64 {
    // Rough estimate: $3/M input, $15/M output (Sonnet-class pricing)
    let input_cost = usage.prompt_tokens as f64 * 3.0 / 1_000_000.0;
    let output_cost = usage.completion_tokens as f64 * 15.0 / 1_000_000.0;
    input_cost + output_cost
}

#[cfg(test)]
mod tests {
    use super::*;
    use a3s_code::agent::AgentEvent;
    use a3s_code::llm::TokenUsage;

    #[test]
    fn test_translate_text_delta() {
        let event = AgentEvent::TextDelta {
            text: "Hello".to_string(),
        };
        let msgs = translate_event(&event);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            BrowserIncomingMessage::StreamEvent { event, .. } => {
                assert_eq!(event["delta"]["text"], "Hello");
            }
            _ => panic!("Expected StreamEvent"),
        }
    }

    #[test]
    fn test_translate_tool_start() {
        let event = AgentEvent::ToolStart {
            id: "t1".to_string(),
            name: "Bash".to_string(),
        };
        let msgs = translate_event(&event);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            BrowserIncomingMessage::StreamEvent { event, .. } => {
                assert_eq!(event["content_block"]["name"], "Bash");
                assert_eq!(event["content_block"]["id"], "t1");
            }
            _ => panic!("Expected StreamEvent"),
        }
    }

    #[test]
    fn test_translate_tool_end() {
        let event = AgentEvent::ToolEnd {
            id: "t1".to_string(),
            name: "Bash".to_string(),
            output: "ok".to_string(),
            exit_code: 0,
        };
        let msgs = translate_event(&event);
        assert_eq!(msgs.len(), 1);
        assert!(matches!(
            msgs[0],
            BrowserIncomingMessage::ToolProgress { .. }
        ));
    }

    #[test]
    fn test_translate_end() {
        let event = AgentEvent::End {
            text: "Done".to_string(),
            usage: TokenUsage::default(),
        };
        let msgs = translate_event(&event);
        assert_eq!(msgs.len(), 2);
        assert!(matches!(msgs[0], BrowserIncomingMessage::Result { .. }));
        assert!(matches!(
            msgs[1],
            BrowserIncomingMessage::StatusChange { .. }
        ));
    }

    #[test]
    fn test_translate_error() {
        let event = AgentEvent::Error {
            message: "oops".to_string(),
        };
        let msgs = translate_event(&event);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            BrowserIncomingMessage::Error { message } => assert_eq!(message, "oops"),
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_translate_confirmation_required() {
        let event = AgentEvent::ConfirmationRequired {
            tool_id: "t1".to_string(),
            tool_name: "Bash".to_string(),
            args: serde_json::json!({"command": "rm -rf /"}),
            timeout_ms: 30000,
        };
        let msgs = translate_event(&event);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            BrowserIncomingMessage::PermissionRequest { request } => {
                assert_eq!(request.request_id, "t1");
                assert_eq!(request.tool_name, "Bash");
            }
            _ => panic!("Expected PermissionRequest"),
        }
    }

    #[test]
    fn test_translate_confirmation_received() {
        let event = AgentEvent::ConfirmationReceived {
            tool_id: "t1".to_string(),
            approved: true,
            reason: None,
        };
        let msgs = translate_event(&event);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            BrowserIncomingMessage::PermissionCancelled { request_id } => {
                assert_eq!(request_id, "t1");
            }
            _ => panic!("Expected PermissionCancelled"),
        }
    }

    #[test]
    fn test_translate_permission_denied() {
        let event = AgentEvent::PermissionDenied {
            tool_id: "t1".to_string(),
            tool_name: "Bash".to_string(),
            args: serde_json::json!({}),
            reason: "not allowed".to_string(),
        };
        let msgs = translate_event(&event);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            BrowserIncomingMessage::Error { message } => {
                assert!(message.contains("not allowed"));
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_translate_start() {
        let event = AgentEvent::Start {
            prompt: "hi".to_string(),
        };
        let msgs = translate_event(&event);
        assert_eq!(msgs.len(), 1);
        assert!(matches!(
            msgs[0],
            BrowserIncomingMessage::StatusChange { .. }
        ));
    }

    #[test]
    fn test_translate_turn_end() {
        let event = AgentEvent::TurnEnd {
            turn: 3,
            usage: TokenUsage {
                prompt_tokens: 1000,
                completion_tokens: 500,
                ..Default::default()
            },
        };
        let msgs = translate_event(&event);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            BrowserIncomingMessage::SessionUpdate { session } => {
                assert_eq!(session["num_turns"], 3);
            }
            _ => panic!("Expected SessionUpdate"),
        }
    }

    #[test]
    fn test_translate_internal_events_suppressed() {
        let events = vec![
            AgentEvent::TurnStart { turn: 1 },
            AgentEvent::ContextResolving { providers: vec![] },
            AgentEvent::ContextResolved {
                total_items: 0,
                total_tokens: 0,
            },
            AgentEvent::PlanningStart {
                prompt: "test".to_string(),
            },
        ];

        for event in events {
            let msgs = translate_event(&event);
            assert!(msgs.is_empty(), "Expected no messages for {:?}", event);
        }
    }

    #[test]
    fn test_permission_mode_to_policy() {
        let strict = permission_mode_to_policy("strict");
        assert!(strict.enabled);

        let permissive = permission_mode_to_policy("yolo");
        assert!(permissive.enabled);

        let default = permission_mode_to_policy("default");
        assert!(default.enabled);
    }

    #[test]
    fn test_estimate_cost() {
        let usage = TokenUsage {
            prompt_tokens: 1_000_000,
            completion_tokens: 100_000,
            ..Default::default()
        };
        let cost = estimate_cost_from_usage(&usage);
        // 1M input * $3/M + 100K output * $15/M = $3 + $1.50 = $4.50
        assert!((cost - 4.5).abs() < 0.01);
    }
}
