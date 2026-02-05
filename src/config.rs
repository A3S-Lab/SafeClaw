//! SafeClaw configuration management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main SafeClaw configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeClawConfig {
    /// Gateway configuration
    pub gateway: GatewayConfig,

    /// Channel configurations
    pub channels: ChannelsConfig,

    /// TEE configuration
    pub tee: TeeConfig,

    /// Privacy configuration
    pub privacy: PrivacyConfig,

    /// Model configuration
    pub models: ModelsConfig,

    /// Storage configuration
    pub storage: StorageConfig,
}

impl Default for SafeClawConfig {
    fn default() -> Self {
        Self {
            gateway: GatewayConfig::default(),
            channels: ChannelsConfig::default(),
            tee: TeeConfig::default(),
            privacy: PrivacyConfig::default(),
            models: ModelsConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

/// Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Host to bind to
    pub host: String,

    /// Port to listen on
    pub port: u16,

    /// Enable TLS
    pub tls_enabled: bool,

    /// TLS certificate path
    pub tls_cert: Option<PathBuf>,

    /// TLS key path
    pub tls_key: Option<PathBuf>,

    /// WebSocket ping interval in seconds
    pub ws_ping_interval: u64,

    /// Maximum concurrent connections
    pub max_connections: usize,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 18790,
            tls_enabled: false,
            tls_cert: None,
            tls_key: None,
            ws_ping_interval: 30,
            max_connections: 1000,
        }
    }
}

/// Channel configurations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelsConfig {
    /// Telegram channel config
    pub telegram: Option<TelegramConfig>,

    /// Slack channel config
    pub slack: Option<SlackConfig>,

    /// Discord channel config
    pub discord: Option<DiscordConfig>,

    /// WebChat channel config
    pub webchat: Option<WebChatConfig>,
}

/// Telegram channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot token (stored in TEE)
    pub bot_token_ref: String,

    /// Allowed user IDs
    pub allowed_users: Vec<i64>,

    /// DM policy: "pairing" or "open"
    pub dm_policy: String,
}

/// Slack channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Bot token reference (stored in TEE)
    pub bot_token_ref: String,

    /// App token reference (stored in TEE)
    pub app_token_ref: String,

    /// Allowed workspace IDs
    pub allowed_workspaces: Vec<String>,

    /// DM policy
    pub dm_policy: String,
}

/// Discord channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Bot token reference (stored in TEE)
    pub bot_token_ref: String,

    /// Allowed guild IDs
    pub allowed_guilds: Vec<u64>,

    /// DM policy
    pub dm_policy: String,
}

/// WebChat channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebChatConfig {
    /// Enable WebChat
    pub enabled: bool,

    /// Require authentication
    pub require_auth: bool,

    /// Allowed origins for CORS
    pub allowed_origins: Vec<String>,
}

impl Default for WebChatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require_auth: true,
            allowed_origins: vec!["http://localhost:*".to_string()],
        }
    }
}

/// TEE configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeeConfig {
    /// Enable TEE mode
    pub enabled: bool,

    /// TEE backend type
    pub backend: TeeBackend,

    /// A3S Box image reference
    pub box_image: String,

    /// Memory allocation for TEE in MB
    pub memory_mb: u32,

    /// CPU cores for TEE
    pub cpu_cores: u32,

    /// Vsock port for communication
    pub vsock_port: u32,

    /// Attestation configuration
    pub attestation: AttestationConfig,
}

impl Default for TeeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: TeeBackend::A3sBox,
            box_image: "ghcr.io/a3s-lab/safeclaw-tee:latest".to_string(),
            memory_mb: 2048,
            cpu_cores: 2,
            vsock_port: 4089,
            attestation: AttestationConfig::default(),
        }
    }
}

/// TEE backend type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TeeBackend {
    /// A3S Box MicroVM (default)
    #[default]
    A3sBox,

    /// Intel SGX
    IntelSgx,

    /// AMD SEV
    AmdSev,

    /// ARM TrustZone
    ArmTrustzone,
}

/// Attestation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationConfig {
    /// Enable remote attestation
    pub enabled: bool,

    /// Attestation provider
    pub provider: String,

    /// Expected measurements
    pub expected_measurements: HashMap<String, String>,
}

impl Default for AttestationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "local".to_string(),
            expected_measurements: HashMap::new(),
        }
    }
}

/// Privacy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Enable automatic privacy classification
    pub auto_classify: bool,

    /// Default sensitivity level for unclassified data
    pub default_level: SensitivityLevel,

    /// Classification rules
    pub rules: Vec<ClassificationRule>,

    /// Data retention policy
    pub retention: RetentionConfig,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            auto_classify: true,
            default_level: SensitivityLevel::Normal,
            rules: default_classification_rules(),
            retention: RetentionConfig::default(),
        }
    }
}

/// Sensitivity level for data classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SensitivityLevel {
    /// Public data - can be processed anywhere
    Public,

    /// Normal data - default level
    #[default]
    Normal,

    /// Sensitive data - should be processed in TEE
    Sensitive,

    /// Highly sensitive - must be processed in TEE with extra protection
    HighlySensitive,
}

/// Classification rule for privacy detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationRule {
    /// Rule name
    pub name: String,

    /// Pattern to match (regex)
    pub pattern: String,

    /// Sensitivity level to assign
    pub level: SensitivityLevel,

    /// Description
    pub description: String,
}

pub fn default_classification_rules() -> Vec<ClassificationRule> {
    vec![
        ClassificationRule {
            name: "credit_card".to_string(),
            pattern: r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b".to_string(),
            level: SensitivityLevel::HighlySensitive,
            description: "Credit card numbers".to_string(),
        },
        ClassificationRule {
            name: "ssn".to_string(),
            pattern: r"\b\d{3}-\d{2}-\d{4}\b".to_string(),
            level: SensitivityLevel::HighlySensitive,
            description: "Social Security Numbers".to_string(),
        },
        ClassificationRule {
            name: "email".to_string(),
            pattern: r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b".to_string(),
            level: SensitivityLevel::Sensitive,
            description: "Email addresses".to_string(),
        },
        ClassificationRule {
            name: "phone".to_string(),
            pattern: r"\b\+?1?[-.\s]?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b".to_string(),
            level: SensitivityLevel::Sensitive,
            description: "Phone numbers".to_string(),
        },
        ClassificationRule {
            name: "api_key".to_string(),
            pattern: r"\b(sk-|api[_-]?key|token)[A-Za-z0-9_-]{20,}\b".to_string(),
            level: SensitivityLevel::HighlySensitive,
            description: "API keys and tokens".to_string(),
        },
    ]
}

/// Data retention configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Retention period for normal data in days
    pub normal_days: u32,

    /// Retention period for sensitive data in days
    pub sensitive_days: u32,

    /// Enable automatic cleanup
    pub auto_cleanup: bool,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            normal_days: 30,
            sensitive_days: 7,
            auto_cleanup: true,
        }
    }
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    /// Default model provider
    pub default_provider: String,

    /// Model configurations by provider
    pub providers: HashMap<String, ModelProviderConfig>,
}

impl Default for ModelsConfig {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert(
            "anthropic".to_string(),
            ModelProviderConfig {
                api_key_ref: "anthropic_api_key".to_string(),
                base_url: None,
                default_model: "claude-sonnet-4-20250514".to_string(),
                models: vec![
                    "claude-opus-4-20250514".to_string(),
                    "claude-sonnet-4-20250514".to_string(),
                    "claude-haiku-3-5-20241022".to_string(),
                ],
            },
        );
        providers.insert(
            "openai".to_string(),
            ModelProviderConfig {
                api_key_ref: "openai_api_key".to_string(),
                base_url: None,
                default_model: "gpt-4o".to_string(),
                models: vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                    "o1".to_string(),
                ],
            },
        );

        Self {
            default_provider: "anthropic".to_string(),
            providers,
        }
    }
}

/// Model provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProviderConfig {
    /// API key reference (stored in TEE)
    pub api_key_ref: String,

    /// Custom base URL
    pub base_url: Option<String>,

    /// Default model
    pub default_model: String,

    /// Available models
    pub models: Vec<String>,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Base directory for storage
    pub base_dir: PathBuf,

    /// Session storage path
    pub sessions_dir: PathBuf,

    /// Secure storage path (in TEE)
    pub secure_dir: PathBuf,

    /// Enable encryption at rest
    pub encrypt_at_rest: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("safeclaw");

        Self {
            sessions_dir: base.join("sessions"),
            secure_dir: base.join("secure"),
            base_dir: base,
            encrypt_at_rest: true,
        }
    }
}

// Helper module for default directories
mod dirs {
    use std::path::PathBuf;

    pub fn data_local_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join("Library/Application Support"))
        }
        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_DATA_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".local/share")))
        }
        #[cfg(target_os = "windows")]
        {
            std::env::var("LOCALAPPDATA").ok().map(PathBuf::from)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SafeClawConfig::default();
        assert_eq!(config.gateway.port, 18790);
        assert!(config.tee.enabled);
        assert!(config.privacy.auto_classify);
    }

    #[test]
    fn test_classification_rules() {
        let rules = default_classification_rules();
        assert!(!rules.is_empty());
        assert!(rules.iter().any(|r| r.name == "credit_card"));
    }
}
