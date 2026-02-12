//! HTTP handlers for the Events API
//!
//! Provides 5 REST endpoints for event management:
//! - GET    /api/v1/events           — list events (paginated, filterable)
//! - GET    /api/v1/events/:id       — event detail
//! - POST   /api/v1/events           — create event
//! - GET    /api/v1/events/counts    — category counts
//! - PUT    /api/v1/events/subscriptions/:personaId — update subscriptions

use crate::events::store::EventStore;
use crate::events::types::*;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

/// Shared state for event handlers
#[derive(Clone)]
pub struct EventsState {
    pub store: Arc<EventStore>,
}

/// Create the events router with all REST endpoints
pub fn events_router(state: EventsState) -> Router {
    Router::new()
        .route("/api/v1/events", get(list_events))
        .route("/api/v1/events", post(create_event))
        .route("/api/v1/events/counts", get(get_counts))
        .route("/api/v1/events/:id", get(get_event))
        .route(
            "/api/v1/events/subscriptions/:persona_id",
            put(update_subscription),
        )
        .with_state(state)
}

// =============================================================================
// Query parameter types
// =============================================================================

#[derive(Debug, Deserialize)]
struct ListEventsQuery {
    category: Option<String>,
    q: Option<String>,
    since: Option<u64>,
    page: Option<u64>,
    #[serde(rename = "perPage")]
    per_page: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct CountsQuery {
    since: Option<u64>,
}

// =============================================================================
// Handlers
// =============================================================================

/// GET /api/v1/events
async fn list_events(
    State(state): State<EventsState>,
    Query(params): Query<ListEventsQuery>,
) -> impl IntoResponse {
    let category = params.category.as_deref().and_then(|c| c.parse().ok());
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);

    let result = state
        .store
        .list_events(
            category.as_ref(),
            params.q.as_deref(),
            params.since,
            page,
            per_page,
        )
        .await;

    Json(result)
}

/// GET /api/v1/events/:id
async fn get_event(
    State(state): State<EventsState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.store.get_event(&id).await {
        Some(event) => (StatusCode::OK, Json(serde_json::to_value(event).unwrap())),
        None => (
            StatusCode::NOT_FOUND,
            Json(
                serde_json::to_value(ApiError::not_found(format!("Event {} not found", id)))
                    .unwrap(),
            ),
        ),
    }
}

/// POST /api/v1/events
async fn create_event(
    State(state): State<EventsState>,
    Json(request): Json<CreateEventRequest>,
) -> impl IntoResponse {
    let event = state.store.create_event(request).await;
    (
        StatusCode::CREATED,
        Json(serde_json::to_value(event).unwrap()),
    )
}

/// GET /api/v1/events/counts
async fn get_counts(
    State(state): State<EventsState>,
    Query(params): Query<CountsQuery>,
) -> impl IntoResponse {
    let counts = state.store.counts(params.since).await;
    Json(counts)
}

/// PUT /api/v1/events/subscriptions/:personaId
async fn update_subscription(
    State(state): State<EventsState>,
    Path(persona_id): Path<String>,
    Json(request): Json<UpdateSubscriptionRequest>,
) -> impl IntoResponse {
    let sub = state
        .store
        .update_subscription(&persona_id, request.categories)
        .await;
    Json(sub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tempfile::TempDir;
    use tower::ServiceExt;

    async fn make_app() -> (Router, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = Arc::new(EventStore::new(dir.path().to_path_buf()).await.unwrap());
        let state = EventsState { store };
        (events_router(state), dir)
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_body(), 1024 * 64)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn test_list_events_empty() {
        let (app, _dir) = make_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/events")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["data"].as_array().unwrap().len(), 0);
        assert_eq!(json["pagination"]["total"], 0);
    }

    #[tokio::test]
    async fn test_create_and_get_event() {
        let (app, _dir) = make_app().await;

        // Create
        let create_body = serde_json::json!({
            "category": "market",
            "topic": "forex.usd_cny",
            "summary": "USD/CNY broke through 7.35",
            "detail": "Exchange rate: 7.3521",
            "source": "Reuters",
            "subscribers": ["analyst"]
        });

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/events")
                    .header("content-type", "application/json")
                    .body(Body::from(create_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let created = body_json(resp).await;
        let event_id = created["id"].as_str().unwrap();
        assert!(event_id.starts_with("evt-"));
        assert_eq!(created["category"], "market");
        assert_eq!(created["reacted"], false);

        // Get by ID
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/events/{}", event_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let fetched = body_json(resp).await;
        assert_eq!(fetched["id"], event_id);
        assert_eq!(fetched["topic"], "forex.usd_cny");
    }

    #[tokio::test]
    async fn test_get_event_not_found() {
        let (app, _dir) = make_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/events/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let json = body_json(resp).await;
        assert_eq!(json["error"]["code"], "NOT_FOUND");
    }

    #[tokio::test]
    async fn test_list_events_with_category_filter() {
        let (app, _dir) = make_app().await;

        // Create two events with different categories
        for (cat, topic) in [("market", "forex"), ("system", "deploy")] {
            let body = serde_json::json!({
                "category": cat,
                "topic": topic,
                "summary": "test",
                "detail": "test",
                "source": "test"
            });
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/events")
                        .header("content-type", "application/json")
                        .body(Body::from(body.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        // Filter by market
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/events?category=market")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["data"].as_array().unwrap().len(), 1);
        assert_eq!(json["data"][0]["category"], "market");
    }

    #[tokio::test]
    async fn test_list_events_pagination() {
        let (app, _dir) = make_app().await;

        // Create 3 events
        for i in 0..3 {
            let body = serde_json::json!({
                "category": "market",
                "topic": format!("topic-{}", i),
                "summary": "test",
                "detail": "test",
                "source": "test"
            });
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/events")
                        .header("content-type", "application/json")
                        .body(Body::from(body.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/events?page=1&perPage=2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(resp).await;
        assert_eq!(json["data"].as_array().unwrap().len(), 2);
        assert_eq!(json["pagination"]["total"], 3);
        assert_eq!(json["pagination"]["totalPages"], 2);
    }

    #[tokio::test]
    async fn test_get_counts() {
        let (app, _dir) = make_app().await;

        // Create events
        for cat in ["market", "market", "system"] {
            let body = serde_json::json!({
                "category": cat,
                "topic": "t",
                "summary": "s",
                "detail": "d",
                "source": "src"
            });
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/events")
                        .header("content-type", "application/json")
                        .body(Body::from(body.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/events/counts")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["market"], 2);
        assert_eq!(json["system"], 1);
        assert_eq!(json["total"], 3);
    }

    #[tokio::test]
    async fn test_update_subscription() {
        let (app, _dir) = make_app().await;

        let body = serde_json::json!({
            "categories": ["market", "compliance"]
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/events/subscriptions/financial-analyst")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["personaId"], "financial-analyst");
        assert_eq!(json["categories"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_create_event_invalid_body() {
        let (app, _dir) = make_app().await;

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/events")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Missing required fields → 422 Unprocessable Entity (axum default)
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
