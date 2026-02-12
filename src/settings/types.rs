//! Settings wire types
//!
//! Defines the API-facing settings schema with camelCase JSON serialization.
//! API keys are masked in responses (first 8 + last 4 characters visible).

use serde::{Deserialize, Serialize};

/// Settings response (API keys masked)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsResponse {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    pub gateway: GatewaySettings,
    pub privacy: PrivacySettings,
    pub storage: StorageSettings,
}

/// Gateway settings subset
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewaySettings {
    pub listen_addr: String,
    pub tee_enabled: bool,
    pub cors_origins: Vec<String>,
}

/// Privacy settings subset
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacySettings {
    pub classification_enabled: bool,
    pub redaction_enabled: bool,
}

/// Storage settings subset
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSettings {
    pub backend: String,
    pub sessions_dir: String,
}

/// Partial update request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsRequest {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

/// Server info response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub version: String,
    pub rust_version: String,
    pub os: String,
    pub uptime: u64,
    pub sessions_dir: String,
    pub features: ServerFeatures,
}

/// Server feature flags
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerFeatures {
    pub tee: bool,
    pub privacy: bool,
    pub gateway: bool,
}

/// Mask an API key for display: show first 8 + last 4 chars
pub fn mask_api_key(key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    if key.len() <= 12 {
        return "****".to_string();
    }
    format!("{}****{}", &key[..8], &key[key.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key_normal() {
        assert_eq!(mask_api_key("sk-ant-api03-abcdef1234567890"), "sk-ant-a****7890");
    }

    #[test]
    fn test_mask_api_key_short() {
        assert_eq!(mask_api_key("short"), "****");
        assert_eq!(mask_api_key("exactly12ch"), "****");
    }

    #[test]
    fn test_mask_api_key_empty() {
        assert_eq!(mask_api_key(""), "");
    }

    #[test]
    fn test_mask_api_key_13_chars() {
        assert_eq!(mask_api_key("1234567890abc"), "12345678****0abc");
    }

    #[test]
    fn test_settings_response_serialization() {
        let resp = SettingsResponse {
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            base_url: "".to_string(),
            api_key: "sk-ant-****7f3a".to_string(),
            gateway: GatewaySettings {
                listen_addr: "127.0.0.1:18790".to_string(),
                tee_enabled: false,
                cors_origins: vec!["http://localhost:1420".to_string()],
            },
            privacy: PrivacySettings {
                classification_enabled: true,
                redaction_enabled: false,
            },
            storage: StorageSettings {
                backend: "file".to_string(),
                sessions_dir: "~/.safeclaw/sessions".to_string(),
            },
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"provider\":\"anthropic\""));
        assert!(json.contains("\"listenAddr\":\"127.0.0.1:18790\""));
        assert!(json.contains("\"classificationEnabled\":true"));
        assert!(json.contains("\"sessionsDir\""));
    }

    #[test]
    fn test_update_settings_request() {
        let json = r#"{"provider":"openai","model":"gpt-4o"}"#;
        let req: UpdateSettingsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.provider.as_deref(), Some("openai"));
        assert_eq!(req.model.as_deref(), Some("gpt-4o"));
        assert!(req.api_key.is_none());
        assert!(req.base_url.is_none());
    }

    #[test]
    fn test_update_settings_request_with_api_key() {
        let json = r#"{"apiKey":"sk-new-key-12345"}"#;
        let req: UpdateSettingsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.api_key.as_deref(), Some("sk-new-key-12345"));
    }

    #[test]
    fn test_server_info_serialization() {
        let info = ServerInfo {
            version: "0.1.0".to_string(),
            rust_version: "1.83.0".to_string(),
            os: "macos-aarch64".to_string(),
            uptime: 86400,
            sessions_dir: "/home/user/.safeclaw/sessions".to_string(),
            features: ServerFeatures {
                tee: false,
                privacy: true,
                gateway: true,
            },
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"version\":\"0.1.0\""));
        assert!(json.contains("\"rustVersion\":\"1.83.0\""));
        assert!(json.contains("\"uptime\":86400"));
    }
}
