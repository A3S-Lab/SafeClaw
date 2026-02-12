//! HTTP handlers for the Settings API
//!
//! Provides 4 REST endpoints:
//! - GET    /api/v1/settings       — get settings (API keys masked)
//! - PATCH  /api/v1/settings       — update settings
//! - POST   /api/v1/settings/reset — reset to defaults
//! - GET    /api/v1/settings/info  — server info

use crate::config::SafeClawConfig;
use crate::settings::types::*;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Shared state for settings handlers
#[derive(Clone)]
pub struct SettingsState {
    pub config: Arc<RwLock<SafeClawConfig>>,
    /// Raw API keys (not masked) — stored separately for update logic
    pub api_keys: Arc<RwLock<std::collections::HashMap<String, String>>>,
    pub started_at: Instant,
}

/// Create the settings router with all REST endpoints
pub fn settings_router(state: SettingsState) -> Router {
    Router::new()
        .route("/api/v1/settings", get(get_settings))
        .route("/api/v1/settings", patch(update_settings))
        .route("/api/v1/settings/reset", post(reset_settings))
        .route("/api/v1/settings/info", get(get_info))
        .with_state(state)
}

// =============================================================================
// Handlers
// =============================================================================

/// GET /api/v1/settings
async fn get_settings(State(state): State<SettingsState>) -> impl IntoResponse {
    let config = state.config.read().await;
    let keys = state.api_keys.read().await;
    let resp = build_settings_response(&config, &keys);
    Json(resp)
}

/// PATCH /api/v1/settings
async fn update_settings(
    State(state): State<SettingsState>,
    Json(request): Json<UpdateSettingsRequest>,
) -> impl IntoResponse {
    let mut config = state.config.write().await;
    let mut keys = state.api_keys.write().await;

    if let Some(provider) = &request.provider {
        config.models.default_provider = provider.clone();
    }

    if let Some(model) = &request.model {
        let provider_name = config.models.default_provider.clone();
        if let Some(provider_cfg) = config.models.providers.get_mut(&provider_name) {
            provider_cfg.default_model = model.clone();
        }
    }

    if let Some(base_url) = &request.base_url {
        let provider_name = config.models.default_provider.clone();
        if let Some(provider_cfg) = config.models.providers.get_mut(&provider_name) {
            provider_cfg.base_url = if base_url.is_empty() {
                None
            } else {
                Some(base_url.clone())
            };
        }
    }

    if let Some(api_key) = &request.api_key {
        let provider_name = config.models.default_provider.clone();
        keys.insert(provider_name, api_key.clone());
    }

    let resp = build_settings_response(&config, &keys);
    (StatusCode::OK, Json(resp))
}

/// POST /api/v1/settings/reset
async fn reset_settings(State(state): State<SettingsState>) -> impl IntoResponse {
    let mut config = state.config.write().await;
    let keys = state.api_keys.read().await;

    *config = SafeClawConfig::default();

    let resp = build_settings_response(&config, &keys);
    (StatusCode::OK, Json(resp))
}

/// GET /api/v1/settings/info
async fn get_info(State(state): State<SettingsState>) -> impl IntoResponse {
    let config = state.config.read().await;

    let info = ServerInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        rust_version: rustc_version(),
        os: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        uptime: state.started_at.elapsed().as_secs(),
        sessions_dir: config.storage.sessions_dir.display().to_string(),
        features: ServerFeatures {
            tee: config.tee.enabled,
            privacy: config.privacy.auto_classify,
            gateway: config.a3s_gateway.enabled,
        },
    };

    Json(info)
}

// =============================================================================
// Helpers
// =============================================================================

/// Build a SettingsResponse from config + raw keys (masking API keys)
fn build_settings_response(
    config: &SafeClawConfig,
    keys: &std::collections::HashMap<String, String>,
) -> SettingsResponse {
    let provider_name = &config.models.default_provider;
    let provider_cfg = config.models.providers.get(provider_name);

    let raw_key = keys.get(provider_name).map(|s| s.as_str()).unwrap_or("");
    let masked_key = mask_api_key(raw_key);

    let base_url = provider_cfg
        .and_then(|p| p.base_url.as_deref())
        .unwrap_or("")
        .to_string();

    let default_model = provider_cfg
        .map(|p| p.default_model.as_str())
        .unwrap_or("")
        .to_string();

    let cors_origins = config
        .channels
        .webchat
        .as_ref()
        .map(|w| w.allowed_origins.clone())
        .unwrap_or_default();

    SettingsResponse {
        provider: provider_name.clone(),
        model: default_model,
        base_url,
        api_key: masked_key,
        gateway: GatewaySettings {
            listen_addr: format!("{}:{}", config.gateway.host, config.gateway.port),
            tee_enabled: config.tee.enabled,
            cors_origins,
        },
        privacy: PrivacySettings {
            classification_enabled: config.privacy.auto_classify,
            redaction_enabled: false,
        },
        storage: StorageSettings {
            backend: "file".to_string(),
            sessions_dir: config.storage.sessions_dir.display().to_string(),
        },
    }
}

/// Get rustc version at compile time
fn rustc_version() -> String {
    option_env!("RUSTC_VERSION")
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn make_state() -> SettingsState {
        let config = SafeClawConfig::default();
        let mut keys = std::collections::HashMap::new();
        keys.insert(
            "anthropic".to_string(),
            "sk-ant-api03-abcdef1234567890".to_string(),
        );

        SettingsState {
            config: Arc::new(RwLock::new(config)),
            api_keys: Arc::new(RwLock::new(keys)),
            started_at: Instant::now(),
        }
    }

    fn make_app() -> Router {
        settings_router(make_state())
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_body(), 1024 * 64)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn test_get_settings() {
        let app = make_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["provider"], "anthropic");
        assert_eq!(json["model"], "claude-sonnet-4-20250514");
        // API key should be masked
        let key = json["apiKey"].as_str().unwrap();
        assert!(key.contains("****"));
        assert!(!key.contains("abcdef"));
    }

    #[tokio::test]
    async fn test_get_settings_no_key() {
        let state = SettingsState {
            config: Arc::new(RwLock::new(SafeClawConfig::default())),
            api_keys: Arc::new(RwLock::new(std::collections::HashMap::new())),
            started_at: Instant::now(),
        };
        let app = settings_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(resp).await;
        assert_eq!(json["apiKey"], "");
    }

    #[tokio::test]
    async fn test_update_settings_model() {
        let app = make_app();

        let body = serde_json::json!({
            "model": "claude-opus-4-20250514"
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/settings")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["model"], "claude-opus-4-20250514");
        assert_eq!(json["provider"], "anthropic");
    }

    #[tokio::test]
    async fn test_update_settings_provider() {
        let app = make_app();

        let body = serde_json::json!({
            "provider": "openai"
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/v1/settings")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["provider"], "openai");
        assert_eq!(json["model"], "gpt-4o");
    }

    #[tokio::test]
    async fn test_reset_settings() {
        let state = make_state();
        let app = settings_router(state.clone());

        // Change something first
        {
            let mut config = state.config.write().await;
            config.models.default_provider = "openai".to_string();
        }

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/settings/reset")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        // Should be back to default
        assert_eq!(json["provider"], "anthropic");
    }

    #[tokio::test]
    async fn test_get_info() {
        let app = make_app();

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/settings/info")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json["version"].is_string());
        assert!(json["os"].is_string());
        assert!(json["uptime"].is_number());
        assert!(json["features"]["tee"].is_boolean());
    }
}
