//! HTTP API handler

use crate::error::{Error, Result};
use crate::gateway::Gateway;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// API handler for HTTP endpoints
pub struct ApiHandler {
    gateway: Arc<Gateway>,
}

impl ApiHandler {
    /// Create a new API handler
    pub fn new(gateway: Arc<Gateway>) -> Self {
        Self { gateway }
    }

    /// Create the router
    pub fn router(gateway: Arc<Gateway>) -> Router {
        Router::new()
            .route("/health", get(health_check))
            .route("/status", get(get_status))
            .route("/sessions", get(list_sessions))
            .route("/sessions/:id", get(get_session))
            .route("/message", post(send_message))
            .with_state(gateway)
    }
}

/// Health check response
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Status response
#[derive(Debug, Serialize)]
struct StatusResponse {
    state: String,
    tee_enabled: bool,
    session_count: usize,
    channels: Vec<String>,
}

/// Get gateway status
async fn get_status(State(gateway): State<Arc<Gateway>>) -> impl IntoResponse {
    let state = gateway.state().await;
    let session_count = gateway.session_manager().session_count().await;

    Json(StatusResponse {
        state: format!("{:?}", state),
        tee_enabled: gateway.config().tee.enabled,
        session_count,
        channels: gateway.active_channel_names().await,
    })
}

/// Session info response
#[derive(Debug, Serialize)]
struct SessionInfo {
    id: String,
    user_id: String,
    channel_id: String,
    chat_id: String,
    uses_tee: bool,
    created_at: i64,
    message_count: u64,
}

/// List all sessions
async fn list_sessions(State(gateway): State<Arc<Gateway>>) -> impl IntoResponse {
    let sessions = gateway.session_manager().active_sessions().await;
    let mut infos = Vec::new();

    for session in sessions {
        infos.push(SessionInfo {
            id: session.id.clone(),
            user_id: session.user_id.clone(),
            channel_id: session.channel_id.clone(),
            chat_id: session.chat_id.clone(),
            uses_tee: session.uses_tee,
            created_at: session.created_at,
            message_count: session.message_count().await,
        });
    }

    Json(infos)
}

/// Get a specific session
async fn get_session(
    State(gateway): State<Arc<Gateway>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match gateway.session_manager().get_session(&id).await {
        Some(session) => {
            let info = SessionInfo {
                id: session.id.clone(),
                user_id: session.user_id.clone(),
                channel_id: session.channel_id.clone(),
                chat_id: session.chat_id.clone(),
                uses_tee: session.uses_tee,
                created_at: session.created_at,
                message_count: session.message_count().await,
            };
            (StatusCode::OK, Json(serde_json::to_value(info).unwrap()))
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        ),
    }
}

/// Send message request
#[derive(Debug, Deserialize)]
struct SendMessageRequest {
    channel: String,
    chat_id: String,
    content: String,
}

/// Send message response
#[derive(Debug, Serialize)]
struct SendMessageResponse {
    message_id: String,
    status: String,
}

/// Send a message
async fn send_message(
    State(gateway): State<Arc<Gateway>>,
    Json(request): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let channels = gateway.channels().read().await;
    if let Some(channel) = channels.get(&request.channel) {
        let outbound = crate::channels::OutboundMessage::new(
            &request.channel,
            &request.chat_id,
            &request.content,
        );
        match channel.send_message(outbound).await {
            Ok(message_id) => (
                StatusCode::OK,
                Json(SendMessageResponse {
                    message_id,
                    status: "sent".to_string(),
                }),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SendMessageResponse {
                    message_id: String::new(),
                    status: format!("error: {}", e),
                }),
            ),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(SendMessageResponse {
                message_id: String::new(),
                status: format!("channel '{}' not found", request.channel),
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::GatewayBuilder;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
