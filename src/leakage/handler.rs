//! HTTP handlers for the Audit API
//!
//! Exposes the leakage prevention audit log via REST endpoints:
//! - GET /api/v1/audit/events       — list audit events (paginated, filterable)
//! - GET /api/v1/audit/events/:id   — get single audit event
//! - GET /api/v1/audit/stats        — summary statistics

use crate::error::to_json;
use crate::leakage::audit::{AuditEvent, AuditLog, AuditSeverity};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared state for audit handlers
#[derive(Clone)]
pub struct AuditState {
    pub log: Arc<RwLock<AuditLog>>,
}

/// Create the audit router
pub fn audit_router(state: AuditState) -> Router {
    Router::new()
        .route("/api/v1/audit/events", get(list_events))
        .route("/api/v1/audit/events/:id", get(get_event))
        .route("/api/v1/audit/stats", get(get_stats))
        .with_state(state)
}

// =============================================================================
// Query / Response types
// =============================================================================

#[derive(Debug, Deserialize)]
struct ListEventsQuery {
    session: Option<String>,
    severity: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditStatsResponse {
    total_recorded: u64,
    buffered: usize,
    by_severity: SeverityCounts,
}

#[derive(Debug, Serialize)]
struct SeverityCounts {
    info: usize,
    warning: usize,
    high: usize,
    critical: usize,
}

// =============================================================================
// Handlers
// =============================================================================

/// GET /api/v1/audit/events
async fn list_events(
    State(state): State<AuditState>,
    Query(params): Query<ListEventsQuery>,
) -> impl IntoResponse {
    let log = state.log.read().await;
    let limit = params.limit.unwrap_or(50).min(500);

    let events: Vec<&AuditEvent> = if let Some(session_id) = &params.session {
        let mut events = log.by_session(session_id);
        events.truncate(limit);
        events
    } else if let Some(severity_str) = &params.severity {
        let severity = match severity_str.as_str() {
            "info" => Some(AuditSeverity::Info),
            "warning" => Some(AuditSeverity::Warning),
            "high" => Some(AuditSeverity::High),
            "critical" => Some(AuditSeverity::Critical),
            _ => None,
        };
        match severity {
            Some(s) => {
                let mut events = log.by_severity(s);
                events.truncate(limit);
                events
            }
            None => log.recent(limit),
        }
    } else {
        log.recent(limit)
    };

    // Clone to release the read lock before serializing
    let owned: Vec<AuditEvent> = events.into_iter().cloned().collect();
    Json(owned)
}

/// GET /api/v1/audit/events/:id
async fn get_event(
    State(state): State<AuditState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let log = state.log.read().await;

    // Search through recent events for the matching ID
    let all = log.recent(log.len());
    match all.into_iter().find(|e| e.id == id) {
        Some(event) => (StatusCode::OK, Json(to_json(event))),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": {"code": "NOT_FOUND", "message": format!("Audit event {} not found", id)}})),
        ),
    }
}

/// GET /api/v1/audit/stats
async fn get_stats(State(state): State<AuditState>) -> impl IntoResponse {
    let log = state.log.read().await;

    Json(AuditStatsResponse {
        total_recorded: log.total_count(),
        buffered: log.len(),
        by_severity: SeverityCounts {
            info: log.by_severity(AuditSeverity::Info).len(),
            warning: log.by_severity(AuditSeverity::Warning).len(),
            high: log.by_severity(AuditSeverity::High).len(),
            critical: log.by_severity(AuditSeverity::Critical).len(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leakage::audit::LeakageVector;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    async fn make_app() -> (Router, Arc<RwLock<AuditLog>>) {
        let log = Arc::new(RwLock::new(AuditLog::new(1000)));
        let state = AuditState { log: log.clone() };
        (audit_router(state), log)
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_body(), 1024 * 64)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    async fn seed_events(log: &Arc<RwLock<AuditLog>>) {
        let mut l = log.write().await;
        l.record(AuditEvent::new(
            "sess-1".to_string(),
            AuditSeverity::High,
            LeakageVector::OutputChannel,
            "Tainted data in output".to_string(),
        ));
        l.record(AuditEvent::new(
            "sess-1".to_string(),
            AuditSeverity::Critical,
            LeakageVector::ToolCall,
            "Blocked tool call with secret".to_string(),
        ));
        l.record(AuditEvent::new(
            "sess-2".to_string(),
            AuditSeverity::Warning,
            LeakageVector::DangerousCommand,
            "Dangerous command detected".to_string(),
        ));
    }

    #[tokio::test]
    async fn test_list_events_empty() {
        let (app, _log) = make_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/audit/events")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_list_events_all() {
        let (app, log) = make_app().await;
        seed_events(&log).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/audit/events")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json.as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn test_list_events_by_session() {
        let (app, log) = make_app().await;
        seed_events(&log).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/audit/events?session=sess-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(resp).await;
        assert_eq!(json.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_list_events_by_severity() {
        let (app, log) = make_app().await;
        seed_events(&log).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/audit/events?severity=critical")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(resp).await;
        assert_eq!(json.as_array().unwrap().len(), 1);
        assert_eq!(json[0]["severity"], "critical");
    }

    #[tokio::test]
    async fn test_get_event_not_found() {
        let (app, _log) = make_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/audit/events/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_event_found() {
        let (app, log) = make_app().await;
        seed_events(&log).await;

        // Get the ID of the first event
        let event_id = {
            let l = log.read().await;
            l.recent(1)[0].id.clone()
        };

        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/audit/events/{}", event_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["id"], event_id);
    }

    #[tokio::test]
    async fn test_stats_empty() {
        let (app, _log) = make_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/audit/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["totalRecorded"], 0);
        assert_eq!(json["buffered"], 0);
    }

    #[tokio::test]
    async fn test_stats_with_events() {
        let (app, log) = make_app().await;
        seed_events(&log).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/audit/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(resp).await;
        assert_eq!(json["totalRecorded"], 3);
        assert_eq!(json["buffered"], 3);
        assert_eq!(json["bySeverity"]["high"], 1);
        assert_eq!(json["bySeverity"]["critical"], 1);
        assert_eq!(json["bySeverity"]["warning"], 1);
    }
}
