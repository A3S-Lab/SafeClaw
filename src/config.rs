//! SafeClaw configuration management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main SafeClaw configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SafeClawConfig {
    /// Gateway configuration
    pub gateway: GatewayConfig,

    /// A3S Gateway integration configuration
    #[serde(default)]
    pub a3s_gateway: A3sGatewayConfig,

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

/// A3S Gateway integration configuration
///
/// When enabled, SafeClaw runs as a backend service behind a3s-gateway.
/// The gateway handles TLS, routing, rate limiting, and authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A3sGatewayConfig {
    /// Enable a3s-gateway integration mode
    pub enabled: bool,

    /// Service name registered in a3s-gateway
    pub service_name: String,

    /// A3S Gateway routing rule for SafeClaw API
    pub api_rule: String,

    /// A3S Gateway routing rule for WebSocket
    pub ws_rule: String,

    /// A3S Gateway routing rule for channel webhooks
    pub webhook_rule: String,

    /// Middlewares to apply via a3s-gateway
    pub middlewares: Vec<String>,

    /// Entrypoints to bind in a3s-gateway
    pub entrypoints: Vec<String>,

    /// Enable conversation affinity (sticky sessions)
    pub conversation_affinity: bool,

    /// Sticky session cookie name
    pub affinity_cookie: String,

    /// Enable token metering via a3s-gateway
    pub token_metering: bool,

    /// Max tokens per minute per user (0 = unlimited)
    pub max_tokens_per_minute: u64,
}

impl Default for A3sGatewayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            service_name: "safeclaw".to_string(),
            api_rule: "PathPrefix(`/safeclaw/api`)".to_string(),
            ws_rule: "Path(`/safeclaw/ws`)".to_string(),
            webhook_rule: "PathPrefix(`/safeclaw/webhook`)".to_string(),
            middlewares: vec!["auth-jwt".to_string(), "rate-limit".to_string()],
            entrypoints: vec!["websecure".to_string()],
            conversation_affinity: true,
            affinity_cookie: "safeclaw_session".to_string(),
            token_metering: true,
            max_tokens_per_minute: 10000,
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

    /// Feishu (Lark) channel config
    pub feishu: Option<FeishuConfig>,

    /// DingTalk channel config
    pub dingtalk: Option<DingTalkConfig>,

    /// WeCom (WeChat Work) channel config
    pub wecom: Option<WeComConfig>,
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

/// Feishu (Lark) channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    /// App ID
    pub app_id: String,

    /// App secret reference (stored in TEE)
    pub app_secret_ref: String,

    /// Encrypt key reference for callback verification (stored in TEE)
    pub encrypt_key_ref: String,

    /// Verification token reference (stored in TEE)
    pub verification_token_ref: String,

    /// Allowed user open_ids (empty = all allowed)
    pub allowed_users: Vec<String>,

    /// DM policy: "pairing" or "open"
    pub dm_policy: String,
}

/// DingTalk channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkConfig {
    /// App key reference (stored in TEE)
    pub app_key_ref: String,

    /// App secret reference (stored in TEE)
    pub app_secret_ref: String,

    /// Robot code identifier
    pub robot_code: String,

    /// Allowed user staffIds (empty = all allowed)
    pub allowed_users: Vec<String>,

    /// DM policy: "pairing" or "open"
    pub dm_policy: String,
}

/// WeCom (WeChat Work) channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeComConfig {
    /// Corp ID
    pub corp_id: String,

    /// Agent ID
    pub agent_id: u32,

    /// Corp secret reference (stored in TEE)
    pub secret_ref: String,

    /// Encoding AES key reference for callback decryption (stored in TEE)
    pub encoding_aes_key_ref: String,

    /// Callback token reference (stored in TEE)
    pub token_ref: String,

    /// Allowed user IDs (empty = all allowed)
    pub allowed_users: Vec<String>,

    /// DM policy: "pairing" or "open"
    pub dm_policy: String,
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

    /// Vsock port for TEE secure channel
    pub vsock_port: u32,

    /// Attestation configuration
    pub attestation: AttestationConfig,

    /// Path to a3s-box-shim binary (None = search PATH)
    #[serde(default)]
    pub shim_path: Option<PathBuf>,

    /// Allow simulated TEE reports (development mode only)
    #[serde(default)]
    pub allow_simulated: bool,

    /// Secrets to inject into TEE on boot
    #[serde(default)]
    pub secrets: Vec<SecretRef>,

    /// Workspace directory to mount into VM
    #[serde(default)]
    pub workspace_dir: Option<PathBuf>,

    /// Socket directory for VM communication
    #[serde(default)]
    pub socket_dir: Option<PathBuf>,
}

/// Reference to a secret to inject into the TEE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretRef {
    /// Secret name (used as key inside TEE)
    pub name: String,

    /// Environment variable to read the secret value from
    pub env_var: String,

    /// Whether to also set as environment variable inside TEE
    #[serde(default = "default_true")]
    pub set_env: bool,
}

fn default_true() -> bool {
    true
}

impl Default for TeeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: TeeBackend::A3sBox,
            box_image: "ghcr.io/a3s-lab/safeclaw-tee:latest".to_string(),
            memory_mb: 2048,
            cpu_cores: 2,
            vsock_port: a3s_transport::ports::TEE_CHANNEL,
            attestation: AttestationConfig::default(),
            shim_path: None,
            allow_simulated: false,
            secrets: Vec::new(),
            workspace_dir: None,
            socket_dir: None,
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

// Re-export from shared a3s-privacy crate (single source of truth)
pub use a3s_privacy::default_classification_rules;
pub use a3s_privacy::ClassificationRule;
pub use a3s_privacy::SensitivityLevel;

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

// =============================================================================
// ModelsConfig → a3s-code CodeConfig mapping
// =============================================================================

/// Derive model family from a model ID.
///
/// Returns a short family string such as "claude-opus", "claude-sonnet",
/// "gpt-4o", etc.  Falls back to the full model ID when unrecognised.
fn infer_family(model_id: &str) -> String {
    if model_id.starts_with("claude-opus") {
        "claude-opus".to_string()
    } else if model_id.starts_with("claude-sonnet-4") {
        "claude-sonnet-4".to_string()
    } else if model_id.starts_with("claude-sonnet-3") || model_id.starts_with("claude-3-5-sonnet") {
        "claude-sonnet-3.5".to_string()
    } else if model_id.starts_with("claude-haiku") || model_id.starts_with("claude-3-5-haiku") {
        "claude-haiku".to_string()
    } else if model_id.starts_with("gpt-4o-mini") {
        "gpt-4o-mini".to_string()
    } else if model_id.starts_with("gpt-4o") {
        "gpt-4o".to_string()
    } else if model_id.starts_with("o1-mini") {
        "o1-mini".to_string()
    } else if model_id == "o1" {
        "o1".to_string()
    } else {
        model_id.to_string()
    }
}

/// Resolve API keys from environment variables.
///
/// For each provider, the `api_key_ref` field names an environment variable
/// (e.g. `"anthropic_api_key"` → reads `$ANTHROPIC_API_KEY`).  We try both
/// the original casing and the UPPER_CASE form.
pub fn resolve_api_keys_from_env(models: &ModelsConfig) -> HashMap<String, String> {
    let mut keys = HashMap::new();
    for (provider_name, cfg) in &models.providers {
        // Try exact ref, then UPPER_CASE
        let val = std::env::var(&cfg.api_key_ref)
            .or_else(|_| std::env::var(cfg.api_key_ref.to_uppercase()));
        if let Ok(key) = val {
            keys.insert(provider_name.clone(), key);
        }
    }
    keys
}

impl ModelsConfig {
    /// Convert SafeClaw's model configuration into an a3s-code `CodeConfig`.
    ///
    /// `resolved_keys` maps provider name → API key string.
    /// `sessions_dir` is the on-disk directory for session persistence.
    pub fn to_code_config(
        &self,
        resolved_keys: &HashMap<String, String>,
        sessions_dir: Option<std::path::PathBuf>,
    ) -> a3s_code::config::CodeConfig {
        use a3s_code::config::{ModelConfig as CodeModelConfig, ProviderConfig as CodeProvider};

        let providers: Vec<CodeProvider> = self
            .providers
            .iter()
            .map(|(name, cfg)| {
                let api_key = resolved_keys.get(name).cloned();
                let models: Vec<CodeModelConfig> = cfg
                    .models
                    .iter()
                    .map(|model_id| {
                        let family = infer_family(model_id);
                        let is_anthropic = family.starts_with("claude");
                        CodeModelConfig {
                            id: model_id.clone(),
                            name: model_id.clone(),
                            family,
                            api_key: None,
                            base_url: None,
                            attachment: is_anthropic,
                            reasoning: false,
                            tool_call: true,
                            temperature: true,
                            release_date: None,
                            modalities: a3s_code::config::ModelModalities::default(),
                            cost: a3s_code::config::ModelCost::default(),
                            limit: a3s_code::config::ModelLimit::default(),
                        }
                    })
                    .collect();
                CodeProvider {
                    name: name.clone(),
                    api_key,
                    base_url: cfg.base_url.clone(),
                    models,
                }
            })
            .collect();

        a3s_code::config::CodeConfig {
            default_provider: Some(self.default_provider.clone()),
            default_model: self
                .providers
                .get(&self.default_provider)
                .map(|p| p.default_model.clone()),
            providers,
            sessions_dir,
            storage_backend: a3s_code::config::StorageBackend::File,
            ..Default::default()
        }
    }
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
                .or_else(|| {
                    std::env::var("HOME")
                        .ok()
                        .map(|h| PathBuf::from(h).join(".local/share"))
                })
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

    #[test]
    fn test_feishu_config_serialize() {
        let config = FeishuConfig {
            app_id: "cli_test123".to_string(),
            app_secret_ref: "feishu_secret".to_string(),
            encrypt_key_ref: "feishu_encrypt".to_string(),
            verification_token_ref: "feishu_token".to_string(),
            allowed_users: vec!["ou_user1".to_string()],
            dm_policy: "pairing".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: FeishuConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.app_id, "cli_test123");
        assert_eq!(deserialized.allowed_users.len(), 1);
    }

    #[test]
    fn test_dingtalk_config_serialize() {
        let config = DingTalkConfig {
            app_key_ref: "dt_key".to_string(),
            app_secret_ref: "dt_secret".to_string(),
            robot_code: "robot123".to_string(),
            allowed_users: vec!["staff1".to_string(), "staff2".to_string()],
            dm_policy: "open".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DingTalkConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.robot_code, "robot123");
        assert_eq!(deserialized.allowed_users.len(), 2);
    }

    #[test]
    fn test_wecom_config_serialize() {
        let config = WeComConfig {
            corp_id: "ww_corp123".to_string(),
            agent_id: 1000001,
            secret_ref: "wc_secret".to_string(),
            encoding_aes_key_ref: "wc_aes".to_string(),
            token_ref: "wc_token".to_string(),
            allowed_users: vec![],
            dm_policy: "pairing".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: WeComConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.corp_id, "ww_corp123");
        assert_eq!(deserialized.agent_id, 1000001);
        assert!(deserialized.allowed_users.is_empty());
    }

    #[test]
    fn test_infer_family() {
        assert_eq!(infer_family("claude-opus-4-20250514"), "claude-opus");
        assert_eq!(infer_family("claude-sonnet-4-20250514"), "claude-sonnet-4");
        assert_eq!(
            infer_family("claude-sonnet-3-5-20241022"),
            "claude-sonnet-3.5"
        );
        assert_eq!(
            infer_family("claude-3-5-sonnet-20241022"),
            "claude-sonnet-3.5"
        );
        assert_eq!(infer_family("claude-haiku-3-5-20241022"), "claude-haiku");
        assert_eq!(infer_family("claude-3-5-haiku-20241022"), "claude-haiku");
        assert_eq!(infer_family("gpt-4o"), "gpt-4o");
        assert_eq!(infer_family("gpt-4o-mini"), "gpt-4o-mini");
        assert_eq!(infer_family("o1"), "o1");
        assert_eq!(infer_family("o1-mini"), "o1-mini");
        assert_eq!(infer_family("custom-model-v2"), "custom-model-v2");
    }

    #[test]
    fn test_to_code_config_basic() {
        let models = ModelsConfig::default();
        let mut keys = HashMap::new();
        keys.insert("anthropic".to_string(), "sk-ant-test".to_string());

        let code_cfg = models.to_code_config(&keys, None);

        assert_eq!(code_cfg.default_provider.as_deref(), Some("anthropic"));
        assert_eq!(
            code_cfg.default_model.as_deref(),
            Some("claude-sonnet-4-20250514")
        );
        assert!(!code_cfg.providers.is_empty());

        // Anthropic provider should have the resolved key
        let anthro = code_cfg.providers.iter().find(|p| p.name == "anthropic");
        assert!(anthro.is_some());
        let anthro = anthro.unwrap();
        assert_eq!(anthro.api_key.as_deref(), Some("sk-ant-test"));
        assert_eq!(anthro.models.len(), 3);
        assert_eq!(anthro.models[0].family, "claude-opus");
    }

    #[test]
    fn test_to_code_config_no_key() {
        let models = ModelsConfig::default();
        let keys = HashMap::new(); // no keys resolved
        let code_cfg = models.to_code_config(&keys, None);

        let anthro = code_cfg.providers.iter().find(|p| p.name == "anthropic");
        assert!(anthro.unwrap().api_key.is_none());
    }

    #[test]
    fn test_to_code_config_with_sessions_dir() {
        let models = ModelsConfig::default();
        let keys = HashMap::new();
        let dir = std::path::PathBuf::from("/tmp/safeclaw-sessions");
        let code_cfg = models.to_code_config(&keys, Some(dir.clone()));

        assert_eq!(code_cfg.sessions_dir, Some(dir));
    }

    #[test]
    fn test_channels_config_with_new_channels() {
        let config = ChannelsConfig {
            feishu: Some(FeishuConfig {
                app_id: "cli_test".to_string(),
                app_secret_ref: "secret".to_string(),
                encrypt_key_ref: "encrypt".to_string(),
                verification_token_ref: "token".to_string(),
                allowed_users: vec![],
                dm_policy: "open".to_string(),
            }),
            dingtalk: Some(DingTalkConfig {
                app_key_ref: "key".to_string(),
                app_secret_ref: "secret".to_string(),
                robot_code: "robot".to_string(),
                allowed_users: vec![],
                dm_policy: "open".to_string(),
            }),
            wecom: Some(WeComConfig {
                corp_id: "corp".to_string(),
                agent_id: 100,
                secret_ref: "secret".to_string(),
                encoding_aes_key_ref: "aes".to_string(),
                token_ref: "token".to_string(),
                allowed_users: vec![],
                dm_policy: "open".to_string(),
            }),
            ..Default::default()
        };
        assert!(config.feishu.is_some());
        assert!(config.dingtalk.is_some());
        assert!(config.wecom.is_some());
        assert!(config.telegram.is_none());
    }
}
