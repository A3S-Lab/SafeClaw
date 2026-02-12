//! HTTP handlers for the Personas and User Profile APIs
//!
//! Provides 5 REST endpoints:
//! - GET    /api/v1/personas       — list all personas
//! - GET    /api/v1/personas/:id   — persona detail
//! - POST   /api/v1/personas       — create custom persona
//! - PATCH  /api/v1/personas/:id   — update persona
//! - GET    /api/v1/user/profile   — current user info

use crate::events::types::ApiError;
use crate::personas::store::PersonaStore;
use crate::personas::types::*;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use std::sync::Arc;

/// Shared state for persona handlers
#[derive(Clone)]
pub struct PersonasState {
    pub store: Arc<PersonaStore>,
}

/// Create the personas + user profile router
pub fn personas_router(state: PersonasState) -> Router {
    Router::new()
        .route("/api/v1/personas", get(list_personas))
        .route("/api/v1/personas", post(create_persona))
        .route("/api/v1/personas/:id", get(get_persona))
        .route("/api/v1/personas/:id", patch(update_persona))
        .route("/api/v1/user/profile", get(get_user_profile))
        .with_state(state)
}

// =============================================================================
// Persona handlers
// =============================================================================

/// GET /api/v1/personas
async fn list_personas(State(state): State<PersonasState>) -> impl IntoResponse {
    let personas = state.store.list().await;
    Json(personas)
}

/// GET /api/v1/personas/:id
async fn get_persona(
    State(state): State<PersonasState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.store.get(&id).await {
        Some(persona) => (
            StatusCode::OK,
            Json(serde_json::to_value(persona).unwrap()),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(
                serde_json::to_value(ApiError::not_found(format!(
                    "Persona '{}' not found",
                    id
                )))
                .unwrap(),
            ),
        ),
    }
}

/// POST /api/v1/personas
async fn create_persona(
    State(state): State<PersonasState>,
    Json(request): Json<CreatePersonaRequest>,
) -> impl IntoResponse {
    match state.store.create(request).await {
        Ok(persona) => (
            StatusCode::CREATED,
            Json(serde_json::to_value(persona).unwrap()),
        ),
        Err(err) => (
            StatusCode::CONFLICT,
            Json(serde_json::to_value(err).unwrap()),
        ),
    }
}

/// PATCH /api/v1/personas/:id
async fn update_persona(
    State(state): State<PersonasState>,
    Path(id): Path<String>,
    Json(request): Json<UpdatePersonaRequest>,
) -> impl IntoResponse {
    match state.store.update(&id, request).await {
        Ok(persona) => (
            StatusCode::OK,
            Json(serde_json::to_value(persona).unwrap()),
        ),
        Err(err) => {
            let status = if err.error.code == "FORBIDDEN" {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::NOT_FOUND
            };
            (status, Json(serde_json::to_value(err).unwrap()))
        }
    }
}

// =============================================================================
// User profile handler
// =============================================================================

/// GET /api/v1/user/profile
///
/// Returns a hardcoded user profile. Will be backed by auth system in the future.
async fn get_user_profile() -> impl IntoResponse {
    Json(UserProfile {
        id: 1,
        nickname: "Admin".to_string(),
        email: "[email]".to_string(),
        avatar: "https://github.com/ghost.png".to_string(),
    })
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
        let store = Arc::new(PersonaStore::new(dir.path().to_path_buf()).await.unwrap());
        let state = PersonasState { store };
        (personas_router(state), dir)
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_body(), 1024 * 64)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn test_list_personas() {
        let (app, _dir) = make_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/personas")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        let arr = json.as_array().unwrap();
        assert!(arr.len() >= 5);
        assert!(arr.iter().any(|p| p["id"] == "financial-analyst"));
    }

    #[tokio::test]
    async fn test_get_persona() {
        let (app, _dir) = make_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/personas/fullstack-engineer")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["id"], "fullstack-engineer");
        assert_eq!(json["builtin"], true);
    }

    #[tokio::test]
    async fn test_get_persona_not_found() {
        let (app, _dir) = make_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/personas/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_create_persona() {
        let (app, _dir) = make_app().await;

        let body = serde_json::json!({
            "name": "Tax Specialist",
            "description": "Corporate tax planning"
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/personas")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        assert_eq!(json["id"], "tax-specialist");
        assert_eq!(json["builtin"], false);
        assert_eq!(json["defaultModel"], "claude-sonnet-4-20250514");
    }

    #[tokio::test]
    async fn test_create_duplicate_persona() {
        let (app, _dir) = make_app().await;

        let body = serde_json::json!({
            "name": "Financial Analyst",
            "description": "Duplicate"
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/personas")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_update_custom_persona() {
        let (app, _dir) = make_app().await;

        // Create first
        let create_body = serde_json::json!({
            "name": "My Agent",
            "description": "Original"
        });
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/personas")
                    .header("content-type", "application/json")
                    .body(Body::from(create_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Update
        let update_body = serde_json::json!({
            "name": "My Updated Agent",
            "systemPrompt": "Updated prompt"
        });
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/personas/my-agent")
                    .header("content-type", "application/json")
                    .body(Body::from(update_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["name"], "My Updated Agent");
        assert_eq!(json["systemPrompt"], "Updated prompt");
    }

    #[tokio::test]
    async fn test_update_builtin_forbidden() {
        let (app, _dir) = make_app().await;

        let body = serde_json::json!({"name": "Hacked"});
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/personas/financial-analyst")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_user_profile() {
        let (app, _dir) = make_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/user/profile")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["id"], 1);
        assert!(json["nickname"].is_string());
        assert!(json["email"].is_string());
    }
}
