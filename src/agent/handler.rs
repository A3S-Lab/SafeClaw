//! HTTP and WebSocket handlers for the agent module
//!
//! Provides REST API endpoints for session management and a WebSocket
//! upgrade handler for browser connections.  All handlers delegate to
//! `AgentEngine` which wraps a3s-code's `SessionManager` in-process.

use crate::agent::engine::AgentEngine;
use crate::agent::types::*;
use crate::config::ModelsConfig;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Shared state for agent handlers
#[derive(Clone)]
pub struct AgentState {
    pub engine: Arc<AgentEngine>,
    pub models: ModelsConfig,
}

/// Create the agent router with all REST and WebSocket endpoints
pub fn agent_router(state: AgentState) -> Router {
    Router::new()
        // REST endpoints
        .route("/api/agent/sessions", post(create_session))
        .route("/api/agent/sessions", get(list_sessions))
        .route("/api/agent/sessions/:id", get(get_session))
        .route("/api/agent/sessions/:id", patch(update_session))
        .route("/api/agent/sessions/:id", delete(delete_session))
        .route("/api/agent/sessions/:id/relaunch", post(relaunch_session))
        .route("/api/agent/backends", get(list_backends))
        // WebSocket endpoint (browser only — no more CLI subprocess)
        .route("/ws/agent/browser/:id", get(ws_browser_upgrade))
        .with_state(state)
}

// =============================================================================
// REST handlers
// =============================================================================

/// Create session request body
#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    model: Option<String>,
    permission_mode: Option<String>,
    cwd: Option<String>,
}

/// Create a new agent session
async fn create_session(
    State(state): State<AgentState>,
    Json(request): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let session_id = uuid::Uuid::new_v4().to_string();

    match state
        .engine
        .create_session(
            &session_id,
            request.model,
            request.permission_mode,
            request.cwd,
        )
        .await
    {
        Ok(info) => (
            StatusCode::CREATED,
            Json(serde_json::to_value(info).unwrap()),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

/// List all agent sessions
async fn list_sessions(State(state): State<AgentState>) -> impl IntoResponse {
    let sessions = state.engine.list_sessions().await;
    Json(sessions)
}

/// Get a specific agent session by ID
async fn get_session(State(state): State<AgentState>, Path(id): Path<String>) -> impl IntoResponse {
    match state.engine.get_session(&id).await {
        Some(info) => (StatusCode::OK, Json(serde_json::to_value(info).unwrap())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        ),
    }
}

/// Update session request body
#[derive(Debug, Deserialize)]
struct UpdateSessionRequest {
    name: Option<String>,
    archived: Option<bool>,
}

/// Update a session's name or archived status
async fn update_session(
    State(state): State<AgentState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateSessionRequest>,
) -> impl IntoResponse {
    if state.engine.get_session(&id).await.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        );
    }

    if let Some(name) = request.name {
        state.engine.set_name(&id, name).await;
    }
    if let Some(archived) = request.archived {
        state.engine.set_archived(&id, archived).await;
    }

    let info = state.engine.get_session(&id).await.unwrap();
    (StatusCode::OK, Json(serde_json::to_value(info).unwrap()))
}

/// Delete a session and remove all state
async fn delete_session(
    State(state): State<AgentState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if state.engine.get_session(&id).await.is_none() {
        return StatusCode::NOT_FOUND;
    }

    let _ = state.engine.destroy_session(&id).await;

    StatusCode::NO_CONTENT
}

/// Relaunch a session (destroy + recreate with same config)
async fn relaunch_session(
    State(state): State<AgentState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let existing = match state.engine.get_session(&id).await {
        Some(info) => info,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Session not found"})),
            );
        }
    };

    // Destroy existing session
    let _ = state.engine.destroy_session(&id).await;

    // Recreate with same config
    match state
        .engine
        .create_session(
            &id,
            existing.model,
            existing.permission_mode,
            Some(existing.cwd),
        )
        .await
    {
        Ok(info) => (StatusCode::OK, Json(serde_json::to_value(info).unwrap())),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        ),
    }
}

/// Available model backend
#[derive(Debug, Serialize)]
struct BackendInfo {
    id: String,
    name: String,
    provider: String,
    is_default: bool,
}

/// Derive a human-readable display name from a model ID.
///
/// Maps well-known model IDs to friendly names; falls back to the raw ID
/// for unrecognized models.
fn model_display_name(model_id: &str) -> String {
    match model_id {
        s if s.starts_with("claude-opus-4") => "Claude Opus 4".to_string(),
        s if s.starts_with("claude-sonnet-4") => "Claude Sonnet 4".to_string(),
        s if s.starts_with("claude-haiku-3-5") | s.starts_with("claude-3-5-haiku") => {
            "Claude 3.5 Haiku".to_string()
        }
        s if s.starts_with("claude-sonnet-3-5") | s.starts_with("claude-3-5-sonnet") => {
            "Claude 3.5 Sonnet".to_string()
        }
        "gpt-4o" => "GPT-4o".to_string(),
        "gpt-4o-mini" => "GPT-4o Mini".to_string(),
        "o1" => "O1".to_string(),
        "o1-mini" => "O1 Mini".to_string(),
        other => other.to_string(),
    }
}

/// List available model backends from configuration
async fn list_backends(State(state): State<AgentState>) -> impl IntoResponse {
    let mut backends = Vec::new();

    for (provider_name, provider_cfg) in &state.models.providers {
        for model_id in &provider_cfg.models {
            let is_default = provider_name == &state.models.default_provider
                && model_id == &provider_cfg.default_model;
            backends.push(BackendInfo {
                id: model_id.clone(),
                name: model_display_name(model_id),
                provider: provider_name.clone(),
                is_default,
            });
        }
    }

    // Sort: default provider first, then alphabetically by provider, then by model id
    backends.sort_by(|a, b| {
        let a_is_default_provider = a.provider == state.models.default_provider;
        let b_is_default_provider = b.provider == state.models.default_provider;
        b_is_default_provider
            .cmp(&a_is_default_provider)
            .then_with(|| a.provider.cmp(&b.provider))
            .then_with(|| a.id.cmp(&b.id))
    });

    Json(backends)
}

// =============================================================================
// WebSocket handler
// =============================================================================

/// WebSocket upgrade handler for browser connections
async fn ws_browser_upgrade(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(state): State<AgentState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_browser_ws(socket, session_id, state))
}

/// Handle browser WebSocket connection (JSON protocol)
///
/// Browser sends/receives JSON messages. On connect, receives session_init,
/// message history, and pending permission requests for state replay.
async fn handle_browser_ws(socket: WebSocket, session_id: String, state: AgentState) {
    let browser_id = uuid::Uuid::new_v4().to_string();
    tracing::info!(
        session_id = %session_id,
        browser_id = %browser_id,
        "Browser WebSocket connected"
    );

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Channel for engine → browser outbound messages
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Register with engine (sends session_init, history, pending permissions)
    let registered = state
        .engine
        .handle_browser_open(&session_id, &browser_id, tx)
        .await;

    if !registered {
        tracing::warn!(
            session_id = %session_id,
            "Browser connected to unknown session"
        );
        let error_msg = serde_json::json!({
            "type": "error",
            "message": "Session not found"
        });
        let _ = ws_sender.send(Message::Text(error_msg.to_string())).await;
        return;
    }

    // Forward engine → browser messages
    let send_session_id = session_id.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(Message::Text(msg)).await.is_err() {
                tracing::debug!(
                    session_id = %send_session_id,
                    "Browser WebSocket send failed"
                );
                break;
            }
        }
    });

    // Receive browser → engine messages (JSON)
    let recv_engine = state.engine.clone();
    let recv_session_id = session_id.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<BrowserOutgoingMessage>(&text) {
                        Ok(browser_msg) => {
                            recv_engine
                                .handle_browser_message(&recv_session_id, browser_msg)
                                .await;
                        }
                        Err(e) => {
                            let preview = &text[..text.len().min(200)];
                            tracing::warn!(
                                session_id = %recv_session_id,
                                "Invalid browser message: {} (raw: {})",
                                e,
                                preview
                            );
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }

    // Clean up browser connection in engine
    state
        .engine
        .handle_browser_close(&session_id, &browser_id)
        .await;

    tracing::info!(
        session_id = %session_id,
        browser_id = %browser_id,
        "Browser WebSocket disconnected"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::engine::AgentEngine;
    use crate::agent::session_store::AgentSessionStore;
    use crate::config::{resolve_api_keys_from_env, ModelProviderConfig};
    use std::collections::HashMap;
    use tempfile::TempDir;

    async fn make_state() -> (AgentState, TempDir) {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();

        let models = ModelsConfig::default();
        let keys = resolve_api_keys_from_env(&models);
        let code_config = models.to_code_config(&keys, Some(dir.path().to_path_buf()));

        let cwd = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
            .to_string_lossy()
            .to_string();
        let tool_executor = Arc::new(a3s_code::tools::ToolExecutor::with_config(
            cwd,
            &code_config,
        ));
        let session_manager = Arc::new(
            a3s_code::session::SessionManager::with_persistence(None, tool_executor, dir.path())
                .await
                .unwrap(),
        );
        let store = Arc::new(AgentSessionStore::new(dir.path().join("ui-state")));
        let engine = Arc::new(
            AgentEngine::new(session_manager, code_config, store)
                .await
                .unwrap(),
        );

        let state = AgentState { engine, models };
        (state, dir)
    }

    #[tokio::test]
    async fn test_agent_state_is_clone() {
        let (state, _dir) = make_state().await;
        let _cloned = state.clone();
    }

    #[tokio::test]
    async fn test_agent_router_builds() {
        let (state, _dir) = make_state().await;
        let _router = agent_router(state);
    }

    #[test]
    fn test_backend_info_serialization() {
        let info = BackendInfo {
            id: "claude-sonnet-4-20250514".to_string(),
            name: "Claude Sonnet 4".to_string(),
            provider: "anthropic".to_string(),
            is_default: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("claude-sonnet-4-20250514"));
        assert!(json.contains("Claude Sonnet 4"));
        assert!(json.contains("\"provider\":\"anthropic\""));
        assert!(json.contains("\"is_default\":true"));
    }

    #[test]
    fn test_model_display_name_known_models() {
        assert_eq!(
            model_display_name("claude-opus-4-20250514"),
            "Claude Opus 4"
        );
        assert_eq!(
            model_display_name("claude-sonnet-4-20250514"),
            "Claude Sonnet 4"
        );
        assert_eq!(
            model_display_name("claude-haiku-3-5-20241022"),
            "Claude 3.5 Haiku"
        );
        assert_eq!(model_display_name("gpt-4o"), "GPT-4o");
        assert_eq!(model_display_name("gpt-4o-mini"), "GPT-4o Mini");
        assert_eq!(model_display_name("o1"), "O1");
    }

    #[test]
    fn test_model_display_name_unknown_falls_back() {
        assert_eq!(model_display_name("my-custom-model"), "my-custom-model");
    }

    #[test]
    fn test_create_session_request_deserialization() {
        let json = r#"{"model":"claude-sonnet-4-20250514","permission_mode":"default"}"#;
        let req: CreateSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.model.as_deref(), Some("claude-sonnet-4-20250514"));
        assert_eq!(req.permission_mode.as_deref(), Some("default"));
        assert!(req.cwd.is_none());
    }

    #[test]
    fn test_create_session_request_minimal() {
        let json = r#"{}"#;
        let req: CreateSessionRequest = serde_json::from_str(json).unwrap();
        assert!(req.model.is_none());
        assert!(req.permission_mode.is_none());
        assert!(req.cwd.is_none());
    }

    #[test]
    fn test_update_session_request_deserialization() {
        let json = r#"{"name":"My Session","archived":true}"#;
        let req: UpdateSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("My Session"));
        assert_eq!(req.archived, Some(true));
    }

    #[test]
    fn test_update_session_request_partial() {
        let json = r#"{"name":"New Name"}"#;
        let req: UpdateSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("New Name"));
        assert!(req.archived.is_none());
    }

    #[tokio::test]
    async fn test_list_backends_returns_config_models() {
        let (state, _dir) = make_state().await;
        let response = list_backends(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024 * 64)
            .await
            .unwrap();
        let backends: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

        // Default ModelsConfig has anthropic (3 models) + openai (3 models) = 6
        assert_eq!(backends.len(), 6);

        // Verify provider field is present on all entries
        for b in &backends {
            assert!(b.get("provider").is_some());
            assert!(b.get("is_default").is_some());
        }

        // Verify the default model is marked
        let defaults: Vec<&serde_json::Value> = backends
            .iter()
            .filter(|b| b["is_default"] == true)
            .collect();
        assert!(!defaults.is_empty());
    }

    #[tokio::test]
    async fn test_list_backends_custom_config() {
        let (mut state, _dir) = make_state().await;
        let mut providers = HashMap::new();
        providers.insert(
            "custom".to_string(),
            ModelProviderConfig {
                api_key_ref: "key".to_string(),
                base_url: None,
                default_model: "my-model-v2".to_string(),
                models: vec!["my-model-v1".to_string(), "my-model-v2".to_string()],
            },
        );
        state.models = ModelsConfig {
            default_provider: "custom".to_string(),
            providers,
        };

        let response = list_backends(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024 * 64)
            .await
            .unwrap();
        let backends: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

        assert_eq!(backends.len(), 2);
        assert_eq!(backends[0]["provider"], "custom");

        // my-model-v2 is the default
        let default_backend = backends.iter().find(|b| b["is_default"] == true).unwrap();
        assert_eq!(default_backend["id"], "my-model-v2");
        // Unknown model should use raw ID as display name
        assert_eq!(default_backend["name"], "my-model-v2");
    }

    #[tokio::test]
    async fn test_list_sessions_empty() {
        let (state, _dir) = make_state().await;
        let response = list_sessions(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let (state, _dir) = make_state().await;
        let response = get_session(State(state), Path("nonexistent".to_string()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_session_not_found() {
        let (state, _dir) = make_state().await;
        let response = delete_session(State(state), Path("nonexistent".to_string()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_relaunch_session_not_found() {
        let (state, _dir) = make_state().await;
        let response = relaunch_session(State(state), Path("nonexistent".to_string()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_session_not_found() {
        let (state, _dir) = make_state().await;
        let req = UpdateSessionRequest {
            name: Some("New Name".to_string()),
            archived: None,
        };
        let response = update_session(State(state), Path("nonexistent".to_string()), Json(req))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_create_and_get_session() {
        let (state, _dir) = make_state().await;

        let req = CreateSessionRequest {
            model: None,
            permission_mode: None,
            cwd: Some("/tmp".to_string()),
        };
        let response = create_session(State(state.clone()), Json(req))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), 1024 * 64)
            .await
            .unwrap();
        let info: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let session_id = info["session_id"].as_str().unwrap().to_string();

        // Should be findable
        let response = get_session(State(state.clone()), Path(session_id.clone()))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        // Delete it
        let response = delete_session(State(state), Path(session_id))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}
