//! Unified API router for SafeClaw
//!
//! Merges all module routers into a single axum `Router` with CORS,
//! consistent error handling, and a shared application state.
//!
//! ## Endpoint Map
//!
//! | Prefix                          | Module   | Description                    |
//! |---------------------------------|----------|--------------------------------|
//! | `/health`                       | gateway  | Load balancer health probe     |
//! | `/.well-known/a3s-service.json` | gateway  | Service discovery              |
//! | `/api/v1/gateway/*`             | gateway  | Status, sessions, message, webhook |
//! | `/api/v1/privacy/*`             | privacy  | Classify, analyze, scan, compliance |
//! | `/api/v1/audit/*`               | leakage  | Audit events, stats            |
//! | `/api/v1/settings/*`            | settings | Configuration CRUD             |
//! | `/api/v1/personas/*`            | personas | Persona management             |
//! | `/api/v1/user/*`                | personas | User profile                   |
//! | `/api/v1/events/*`              | events   | Event feed                     |
//! | `/api/agent/*`                  | agent    | Agent sessions, backends       |
//! | `/ws/agent/browser/:id`         | agent    | Agent WebSocket                |

use crate::agent::{agent_router, AgentState};
use crate::events::{events_router, EventsState};
use crate::gateway::Gateway;
use crate::leakage::audit::AuditLog;
use crate::leakage::handler::{audit_router, AuditState};
use crate::personas::{personas_router, PersonasState};
use crate::privacy::handler::{privacy_router, PrivacyState};
use crate::settings::{settings_router, SettingsState};
use axum::{
    http::{header, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

/// Combined application state holding references to all subsystems
#[derive(Clone)]
pub struct AppState {
    pub gateway: Arc<Gateway>,
    pub audit_log: Arc<RwLock<AuditLog>>,
}

/// Build the complete SafeClaw HTTP application
///
/// Merges all module routers, adds CORS middleware, and returns a single
/// `Router` ready to be served by `axum::serve`.
pub fn build_app(
    gateway: Arc<Gateway>,
    agent_state: AgentState,
    settings_state: SettingsState,
    personas_state: PersonasState,
    events_state: EventsState,
    privacy_state: PrivacyState,
    audit_state: AuditState,
    cors_origins: &[String],
) -> Router {
    let cors = build_cors(cors_origins);

    // Gateway routes (health + service discovery at root, rest under /api/v1/gateway)
    let gateway_routes = gateway_api_router(gateway.clone());

    Router::new()
        // Root-level probes
        .route("/health", get(health_check))
        .route("/.well-known/a3s-service.json", get(service_discovery))
        // Gateway API
        .merge(gateway_routes)
        // Module routers (each defines its own /api/... prefixed routes)
        .merge(agent_router(agent_state))
        .merge(settings_router(settings_state))
        .merge(personas_router(personas_state))
        .merge(events_router(events_state))
        .merge(privacy_router(privacy_state))
        .merge(audit_router(audit_state))
        // CORS
        .layer(cors)
}

// =============================================================================
// Gateway sub-router (re-mounted under /api/v1/gateway)
// =============================================================================

fn gateway_api_router(gateway: Arc<Gateway>) -> Router {
    Router::new()
        .route("/api/v1/gateway/status", get(gateway_status))
        .route("/api/v1/gateway/sessions", get(gateway_list_sessions))
        .route("/api/v1/gateway/sessions/:id", get(gateway_get_session))
        .route("/api/v1/gateway/message", post(gateway_send_message))
        .route(
            "/api/v1/gateway/webhook/:channel",
            post(gateway_webhook),
        )
        .with_state(gateway)
}

// =============================================================================
// Root handlers
// =============================================================================

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn service_discovery() -> impl IntoResponse {
    Json(crate::gateway::build_service_descriptor())
}

// =============================================================================
// Gateway handlers (delegating to Gateway methods)
// =============================================================================

async fn gateway_status(
    axum::extract::State(gateway): axum::extract::State<Arc<Gateway>>,
) -> impl IntoResponse {
    Json(gateway.status().await)
}

async fn gateway_list_sessions(
    axum::extract::State(gateway): axum::extract::State<Arc<Gateway>>,
) -> impl IntoResponse {
    let sessions = gateway.session_manager().active_sessions().await;
    let infos: Vec<serde_json::Value> = futures::future::join_all(sessions.iter().map(|s| async {
        serde_json::json!({
            "id": s.id,
            "userId": s.user_id,
            "channelId": s.channel_id,
            "chatId": s.chat_id,
            "usesTee": s.uses_tee().await,
            "createdAt": s.created_at,
            "messageCount": s.message_count().await,
        })
    }))
    .await;
    Json(infos)
}

async fn gateway_get_session(
    axum::extract::State(gateway): axum::extract::State<Arc<Gateway>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match gateway.session_manager().get_session(&id).await {
        Some(session) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "id": session.id,
                "userId": session.user_id,
                "channelId": session.channel_id,
                "chatId": session.chat_id,
                "usesTee": session.uses_tee().await,
                "createdAt": session.created_at,
                "messageCount": session.message_count().await,
            })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": {"code": "NOT_FOUND", "message": "Session not found"}})),
        ),
    }
}

#[derive(serde::Deserialize)]
struct SendMessageRequest {
    channel: String,
    chat_id: String,
    content: String,
}

async fn gateway_send_message(
    axum::extract::State(gateway): axum::extract::State<Arc<Gateway>>,
    Json(request): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let channels = gateway.channels().read().await;
    match channels.get(&request.channel) {
        Some(channel) => {
            let outbound = crate::channels::OutboundMessage::new(
                &request.channel,
                &request.chat_id,
                &request.content,
            );
            match channel.send_message(outbound).await {
                Ok(message_id) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"messageId": message_id, "status": "sent"})),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": {"code": "SEND_FAILED", "message": e.to_string()}})),
                ),
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": {"code": "NOT_FOUND", "message": format!("Channel '{}' not found", request.channel)}})),
        ),
    }
}

async fn gateway_webhook(
    axum::extract::State(gateway): axum::extract::State<Arc<Gateway>>,
    axum::extract::Path(channel): axum::extract::Path<String>,
    body: String,
) -> impl IntoResponse {
    match gateway.process_webhook(&channel, &body).await {
        Ok(Some(response)) => (StatusCode::OK, Json(serde_json::to_value(response).unwrap())),
        Ok(None) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ignored", "reason": "no actionable content"})),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": {"code": "WEBHOOK_ERROR", "message": e.to_string()}})),
        ),
    }
}

// =============================================================================
// CORS
// =============================================================================

fn build_cors(origins: &[String]) -> CorsLayer {
    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT]);

    if origins.is_empty() {
        cors.allow_origin(Any)
    } else {
        let parsed: Vec<_> = origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        cors.allow_origin(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let resp = health_check().await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_service_discovery() {
        let resp = service_discovery().await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn test_build_cors_empty_origins() {
        let _cors = build_cors(&[]);
    }

    #[test]
    fn test_build_cors_with_origins() {
        let _cors = build_cors(&[
            "http://localhost:1420".to_string(),
            "https://app.example.com".to_string(),
        ]);
    }
}
