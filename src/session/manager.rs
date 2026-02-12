//! Unified session management
//!
//! Provides a single `Session` type that optionally supports TEE processing,
//! and a `SessionManager` that handles both regular and TEE session lifecycles.

use crate::config::{SensitivityLevel, TeeConfig};
use crate::error::{Error, Result};
use crate::tee::TeeClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Session is being created
    Creating,
    /// Session is active
    Active,
    /// Session is processing a message (also covers TEE "Busy")
    Processing,
    /// Session is paused
    Paused,
    /// Session is being terminated
    Terminating,
    /// Session has been terminated
    Terminated,
}

/// Handle to a TEE session associated with a regular session.
#[derive(Debug, Clone)]
pub struct TeeHandle {
    /// TEE-side session identifier
    pub tee_session_id: String,
    /// Client for communicating with the TEE environment
    pub client: Arc<TeeClient>,
}

/// A user session, optionally backed by a TEE environment.
#[derive(Debug)]
pub struct Session {
    /// Session ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Channel ID
    pub channel_id: String,
    /// Chat ID (channel-specific)
    pub chat_id: String,
    /// Current state
    state: Arc<RwLock<SessionState>>,
    /// Highest sensitivity level seen
    sensitivity_level: Arc<RwLock<SensitivityLevel>>,
    /// Creation timestamp
    pub created_at: i64,
    /// Last activity timestamp
    last_activity: Arc<RwLock<i64>>,
    /// Message count
    message_count: Arc<RwLock<u64>>,
    /// Session metadata
    metadata: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// Optional TEE handle (replaces the old uses_tee + tee_session_id fields)
    tee: Arc<RwLock<Option<TeeHandle>>>,
}

impl Session {
    /// Create a new session
    pub fn new(user_id: String, channel_id: String, chat_id: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            channel_id,
            chat_id,
            state: Arc::new(RwLock::new(SessionState::Creating)),
            sensitivity_level: Arc::new(RwLock::new(SensitivityLevel::Normal)),
            created_at: now,
            last_activity: Arc::new(RwLock::new(now)),
            message_count: Arc::new(RwLock::new(0)),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            tee: Arc::new(RwLock::new(None)),
        }
    }

    /// Get current state
    pub async fn state(&self) -> SessionState {
        *self.state.read().await
    }

    /// Set state
    pub async fn set_state(&self, state: SessionState) {
        *self.state.write().await = state;
    }

    /// Check if session is active
    pub async fn is_active(&self) -> bool {
        matches!(
            self.state().await,
            SessionState::Active | SessionState::Processing
        )
    }

    /// Update last activity
    pub async fn touch(&self) {
        *self.last_activity.write().await = chrono::Utc::now().timestamp_millis();
    }

    /// Get last activity timestamp
    pub async fn last_activity(&self) -> i64 {
        *self.last_activity.read().await
    }

    /// Increment message count
    pub async fn increment_messages(&self) {
        *self.message_count.write().await += 1;
    }

    /// Get message count
    pub async fn message_count(&self) -> u64 {
        *self.message_count.read().await
    }

    /// Update sensitivity level (only increases)
    pub async fn update_sensitivity(&self, level: SensitivityLevel) {
        let mut current = self.sensitivity_level.write().await;
        if level as u8 > *current as u8 {
            *current = level;
        }
    }

    /// Get current sensitivity level
    pub async fn sensitivity_level(&self) -> SensitivityLevel {
        *self.sensitivity_level.read().await
    }

    /// Set metadata value
    pub async fn set_metadata(&self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.write().await.insert(key.into(), value);
    }

    /// Get metadata value
    pub async fn get_metadata(&self, key: &str) -> Option<serde_json::Value> {
        self.metadata.read().await.get(key).cloned()
    }

    /// Upgrade this session to use TEE processing.
    ///
    /// Works through `Arc<Session>` because the TEE handle is behind a `RwLock`.
    pub async fn upgrade_to_tee(&self, handle: TeeHandle) {
        *self.tee.write().await = Some(handle);
    }

    /// Check if this session uses TEE
    pub async fn uses_tee(&self) -> bool {
        self.tee.read().await.is_some()
    }

    /// Get a clone of the TEE handle, if present
    pub async fn tee_handle(&self) -> Option<TeeHandle> {
        self.tee.read().await.clone()
    }

    /// Process a message through the TEE environment.
    ///
    /// Returns an error if the session has no TEE handle.
    pub async fn process_in_tee(&self, content: &str) -> Result<String> {
        let handle = self
            .tee
            .read()
            .await
            .clone()
            .ok_or_else(|| Error::Tee("Session has no TEE handle".to_string()))?;

        self.set_state(SessionState::Processing).await;
        self.touch().await;

        let result = handle.client.process_message(&handle.tee_session_id, content).await;

        self.set_state(SessionState::Active).await;

        result
    }
}

/// Unified session manager handling both regular and TEE sessions.
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<Session>>>>,
    /// Sessions indexed by user_id:channel_id:chat_id
    user_sessions: Arc<RwLock<HashMap<String, String>>>,
    /// TEE configuration
    tee_config: TeeConfig,
    /// TEE client for communicating with the secure environment
    tee_client: Arc<TeeClient>,
}

impl SessionManager {
    /// Create a new session manager with TEE configuration
    pub fn new(tee_config: TeeConfig) -> Self {
        let tee_client = Arc::new(TeeClient::new(tee_config.clone()));
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            user_sessions: Arc::new(RwLock::new(HashMap::new())),
            tee_config,
            tee_client,
        }
    }

    /// Initialize the TEE subsystem (connect to TEE environment)
    pub async fn init_tee(&self) -> Result<()> {
        if !self.tee_config.enabled {
            tracing::info!("TEE is disabled, skipping initialization");
            return Ok(());
        }

        tracing::info!("Initializing TEE subsystem");
        self.tee_client.connect().await?;
        tracing::info!("TEE subsystem initialized");

        Ok(())
    }

    /// Shutdown the TEE subsystem
    pub async fn shutdown_tee(&self) -> Result<()> {
        if !self.tee_config.enabled {
            return Ok(());
        }

        tracing::info!("Shutting down TEE subsystem");

        // Terminate all TEE-enabled sessions
        let sessions: Vec<Arc<Session>> = {
            let sessions = self.sessions.read().await;
            sessions.values().cloned().collect()
        };

        for session in sessions {
            if session.uses_tee().await {
                if let Some(handle) = session.tee_handle().await {
                    if let Err(e) = handle.client.terminate_session(&handle.tee_session_id).await {
                        tracing::warn!(
                            "Failed to terminate TEE session {} for session {}: {}",
                            handle.tee_session_id,
                            session.id,
                            e
                        );
                    }
                }
            }
        }

        self.tee_client.disconnect().await?;
        tracing::info!("TEE subsystem shutdown complete");

        Ok(())
    }

    /// Check if TEE is enabled
    pub fn is_tee_enabled(&self) -> bool {
        self.tee_config.enabled
    }

    /// Get a reference to the TEE client
    pub fn tee_client(&self) -> &Arc<TeeClient> {
        &self.tee_client
    }

    /// Create a new session
    pub async fn create_session(
        &self,
        user_id: &str,
        channel_id: &str,
        chat_id: &str,
    ) -> Result<Arc<Session>> {
        let user_key = format!("{}:{}:{}", user_id, channel_id, chat_id);

        // Check for existing active session
        if let Some(session_id) = self.user_sessions.read().await.get(&user_key) {
            if let Some(session) = self.sessions.read().await.get(session_id) {
                if session.is_active().await {
                    return Ok(session.clone());
                }
            }
        }

        // Create new session
        let session = Arc::new(Session::new(
            user_id.to_string(),
            channel_id.to_string(),
            chat_id.to_string(),
        ));
        let session_id = session.id.clone();

        session.set_state(SessionState::Active).await;

        // Store session
        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session.clone());
        self.user_sessions
            .write()
            .await
            .insert(user_key, session_id);

        tracing::info!(
            "Created session {} for user {} on {}:{}",
            session.id,
            user_id,
            channel_id,
            chat_id
        );

        Ok(session)
    }

    /// Get session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<Session>> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Get session for user
    pub async fn get_user_session(
        &self,
        user_id: &str,
        channel_id: &str,
        chat_id: &str,
    ) -> Option<Arc<Session>> {
        let user_key = format!("{}:{}:{}", user_id, channel_id, chat_id);
        let session_id = self.user_sessions.read().await.get(&user_key)?.clone();
        self.get_session(&session_id).await
    }

    /// Upgrade an existing session to use TEE processing.
    ///
    /// Creates a TEE-side session via the TeeClient and attaches the handle.
    pub async fn upgrade_to_tee(&self, session_id: &str) -> Result<()> {
        if !self.tee_config.enabled {
            return Err(Error::Tee("TEE is not enabled".to_string()));
        }

        let session = self
            .get_session(session_id)
            .await
            .ok_or_else(|| Error::Tee(format!("Session {} not found", session_id)))?;

        if session.uses_tee().await {
            return Ok(()); // Already upgraded
        }

        // Initialize TEE-side session
        let tee_session_id = Uuid::new_v4().to_string();
        self.tee_client
            .init_session(&tee_session_id, &session.user_id)
            .await?;

        let handle = TeeHandle {
            tee_session_id: tee_session_id.clone(),
            client: self.tee_client.clone(),
        };

        session.upgrade_to_tee(handle).await;

        tracing::info!(
            "Upgraded session {} to TEE (tee_session={})",
            session_id,
            tee_session_id
        );

        Ok(())
    }

    /// Process a message in TEE for the given session.
    pub async fn process_in_tee(&self, session_id: &str, content: &str) -> Result<String> {
        let session = self
            .get_session(session_id)
            .await
            .ok_or_else(|| Error::Tee(format!("Session {} not found", session_id)))?;

        session.process_in_tee(content).await
    }

    /// Terminate a session
    pub async fn terminate_session(&self, session_id: &str) -> Result<()> {
        let session = match self.sessions.write().await.remove(session_id) {
            Some(s) => s,
            None => return Ok(()),
        };

        session.set_state(SessionState::Terminating).await;

        // Clean up TEE handle if present
        if let Some(handle) = session.tee_handle().await {
            if let Err(e) = handle.client.terminate_session(&handle.tee_session_id).await {
                tracing::warn!(
                    "Failed to terminate TEE session {} for session {}: {}",
                    handle.tee_session_id,
                    session_id,
                    e
                );
            }
        }

        // Remove from user sessions
        let user_key = format!(
            "{}:{}:{}",
            session.user_id, session.channel_id, session.chat_id
        );
        self.user_sessions.write().await.remove(&user_key);

        session.set_state(SessionState::Terminated).await;

        tracing::info!("Terminated session {}", session_id);

        Ok(())
    }

    /// Get all active sessions
    pub async fn active_sessions(&self) -> Vec<Arc<Session>> {
        let sessions = self.sessions.read().await;
        let mut active = Vec::new();

        for session in sessions.values() {
            if session.is_active().await {
                active.push(session.clone());
            }
        }

        active
    }

    /// Get session count
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Clean up inactive sessions
    pub async fn cleanup_inactive(&self, max_idle_ms: i64) -> Result<usize> {
        let now = chrono::Utc::now().timestamp_millis();
        let sessions: Vec<Arc<Session>> = {
            let sessions = self.sessions.read().await;
            sessions.values().cloned().collect()
        };

        let mut cleaned = 0;
        for session in sessions {
            let idle_time = now - session.last_activity().await;
            if idle_time > max_idle_ms {
                if let Err(e) = self.terminate_session(&session.id).await {
                    tracing::warn!("Failed to cleanup session {}: {}", session.id, e);
                } else {
                    cleaned += 1;
                }
            }
        }

        if cleaned > 0 {
            tracing::info!("Cleaned up {} inactive sessions", cleaned);
        }

        Ok(cleaned)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(TeeConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Session tests ----

    #[tokio::test]
    async fn test_session_creation() {
        let session = Session::new(
            "user-123".to_string(),
            "telegram".to_string(),
            "chat-456".to_string(),
        );

        assert_eq!(session.state().await, SessionState::Creating);
        assert!(!session.uses_tee().await);
        assert!(session.tee_handle().await.is_none());
    }

    #[tokio::test]
    async fn test_session_state_transitions() {
        let session = Session::new(
            "user-123".to_string(),
            "telegram".to_string(),
            "chat-456".to_string(),
        );

        session.set_state(SessionState::Active).await;
        assert!(session.is_active().await);

        session.set_state(SessionState::Processing).await;
        assert!(session.is_active().await);

        session.set_state(SessionState::Terminated).await;
        assert!(!session.is_active().await);
    }

    #[tokio::test]
    async fn test_session_sensitivity() {
        let session = Session::new(
            "user-123".to_string(),
            "telegram".to_string(),
            "chat-456".to_string(),
        );

        assert_eq!(session.sensitivity_level().await, SensitivityLevel::Normal);

        session
            .update_sensitivity(SensitivityLevel::Sensitive)
            .await;
        assert_eq!(
            session.sensitivity_level().await,
            SensitivityLevel::Sensitive
        );

        // Should not decrease
        session.update_sensitivity(SensitivityLevel::Normal).await;
        assert_eq!(
            session.sensitivity_level().await,
            SensitivityLevel::Sensitive
        );
    }

    #[tokio::test]
    async fn test_session_uses_tee_default_false() {
        let session = Session::new(
            "user-123".to_string(),
            "telegram".to_string(),
            "chat-456".to_string(),
        );
        assert!(!session.uses_tee().await);
    }

    #[tokio::test]
    async fn test_session_upgrade_to_tee() {
        let session = Session::new(
            "user-123".to_string(),
            "telegram".to_string(),
            "chat-456".to_string(),
        );

        assert!(!session.uses_tee().await);

        let config = TeeConfig::default();
        let client = Arc::new(TeeClient::new(config));
        let handle = TeeHandle {
            tee_session_id: "tee-001".to_string(),
            client,
        };

        session.upgrade_to_tee(handle).await;

        assert!(session.uses_tee().await);
        let h = session.tee_handle().await.unwrap();
        assert_eq!(h.tee_session_id, "tee-001");
    }

    #[tokio::test]
    async fn test_session_upgrade_works_through_arc() {
        let session = Arc::new(Session::new(
            "user-123".to_string(),
            "telegram".to_string(),
            "chat-456".to_string(),
        ));

        assert!(!session.uses_tee().await);

        let config = TeeConfig::default();
        let client = Arc::new(TeeClient::new(config));
        let handle = TeeHandle {
            tee_session_id: "tee-002".to_string(),
            client,
        };

        // This is the key test: upgrade_to_tee works on &self (through Arc)
        session.upgrade_to_tee(handle).await;

        assert!(session.uses_tee().await);
    }

    #[tokio::test]
    async fn test_session_process_in_tee_without_handle() {
        let session = Session::new(
            "user-123".to_string(),
            "telegram".to_string(),
            "chat-456".to_string(),
        );

        let result = session.process_in_tee("hello").await;
        assert!(result.is_err());
    }

    // ---- SessionManager tests ----

    #[tokio::test]
    async fn test_manager_create_session() {
        let manager = SessionManager::default();

        let session = manager
            .create_session("user-123", "telegram", "chat-456")
            .await
            .unwrap();

        assert_eq!(manager.session_count().await, 1);

        // Getting same session should return existing
        let session2 = manager
            .create_session("user-123", "telegram", "chat-456")
            .await
            .unwrap();
        assert_eq!(session.id, session2.id);
        assert_eq!(manager.session_count().await, 1);

        // Different chat should create new session
        let session3 = manager
            .create_session("user-123", "telegram", "chat-789")
            .await
            .unwrap();
        assert_ne!(session.id, session3.id);
        assert_eq!(manager.session_count().await, 2);
    }

    #[tokio::test]
    async fn test_manager_terminate_session() {
        let manager = SessionManager::default();

        let session = manager
            .create_session("user-123", "telegram", "chat-456")
            .await
            .unwrap();
        let session_id = session.id.clone();

        manager.terminate_session(&session_id).await.unwrap();

        assert!(manager.get_session(&session_id).await.is_none());
        assert_eq!(manager.session_count().await, 0);
    }

    #[tokio::test]
    async fn test_manager_tee_disabled_upgrade_fails() {
        let config = TeeConfig {
            enabled: false,
            ..Default::default()
        };
        let manager = SessionManager::new(config);

        let session = manager
            .create_session("user-123", "telegram", "chat-456")
            .await
            .unwrap();

        let result = manager.upgrade_to_tee(&session.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_manager_is_tee_enabled() {
        let disabled = SessionManager::new(TeeConfig {
            enabled: false,
            ..Default::default()
        });
        assert!(!disabled.is_tee_enabled());

        let enabled = SessionManager::new(TeeConfig {
            enabled: true,
            ..Default::default()
        });
        assert!(enabled.is_tee_enabled());
    }

    #[tokio::test]
    async fn test_manager_upgrade_to_tee() {
        let config = TeeConfig {
            enabled: true,
            ..Default::default()
        };
        let manager = SessionManager::new(config);

        // Connect TEE client first (simulated)
        manager.tee_client.connect().await.unwrap();

        let session = manager
            .create_session("user-123", "telegram", "chat-456")
            .await
            .unwrap();

        assert!(!session.uses_tee().await);

        manager.upgrade_to_tee(&session.id).await.unwrap();

        assert!(session.uses_tee().await);
        assert!(session.tee_handle().await.is_some());
    }

    #[tokio::test]
    async fn test_manager_upgrade_nonexistent_session_fails() {
        let config = TeeConfig {
            enabled: true,
            ..Default::default()
        };
        let manager = SessionManager::new(config);

        let result = manager.upgrade_to_tee("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_manager_upgrade_idempotent() {
        let config = TeeConfig {
            enabled: true,
            ..Default::default()
        };
        let manager = SessionManager::new(config);
        manager.tee_client.connect().await.unwrap();

        let session = manager
            .create_session("user-123", "telegram", "chat-456")
            .await
            .unwrap();

        manager.upgrade_to_tee(&session.id).await.unwrap();
        let handle1 = session.tee_handle().await.unwrap();

        // Second upgrade should be a no-op
        manager.upgrade_to_tee(&session.id).await.unwrap();
        let handle2 = session.tee_handle().await.unwrap();

        assert_eq!(handle1.tee_session_id, handle2.tee_session_id);
    }

    #[tokio::test]
    async fn test_manager_process_in_tee() {
        let config = TeeConfig {
            enabled: true,
            ..Default::default()
        };
        let manager = SessionManager::new(config);
        manager.tee_client.connect().await.unwrap();

        let session = manager
            .create_session("user-123", "telegram", "chat-456")
            .await
            .unwrap();

        manager.upgrade_to_tee(&session.id).await.unwrap();

        let result = manager
            .process_in_tee(&session.id, "hello from TEE")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_manager_terminate_tee_session() {
        let config = TeeConfig {
            enabled: true,
            ..Default::default()
        };
        let manager = SessionManager::new(config);
        manager.tee_client.connect().await.unwrap();

        let session = manager
            .create_session("user-123", "telegram", "chat-456")
            .await
            .unwrap();
        let session_id = session.id.clone();

        manager.upgrade_to_tee(&session_id).await.unwrap();
        assert!(session.uses_tee().await);

        // Terminate should clean up TEE handle too
        manager.terminate_session(&session_id).await.unwrap();
        assert!(manager.get_session(&session_id).await.is_none());
    }
}
