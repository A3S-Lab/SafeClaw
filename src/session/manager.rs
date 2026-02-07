//! Session management

use crate::config::SensitivityLevel;
use crate::error::Result;
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
    /// Session is processing a message
    Processing,
    /// Session is paused
    Paused,
    /// Session is being terminated
    Terminating,
    /// Session has been terminated
    Terminated,
}

/// A user session
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
    /// Whether this session uses TEE
    pub uses_tee: bool,
    /// TEE session ID (if using TEE)
    pub tee_session_id: Option<String>,
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
            uses_tee: false,
            tee_session_id: None,
            sensitivity_level: Arc::new(RwLock::new(SensitivityLevel::Normal)),
            created_at: now,
            last_activity: Arc::new(RwLock::new(now)),
            message_count: Arc::new(RwLock::new(0)),
            metadata: Arc::new(RwLock::new(HashMap::new())),
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

    /// Enable TEE for this session
    pub fn enable_tee(&mut self, tee_session_id: String) {
        self.uses_tee = true;
        self.tee_session_id = Some(tee_session_id);
    }
}

/// Session manager
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<Session>>>>,
    /// Sessions indexed by user_id + channel_id + chat_id
    user_sessions: Arc<RwLock<HashMap<String, String>>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            user_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session
    pub async fn create_session(
        &self,
        user_id: &str,
        channel_id: &str,
        chat_id: &str,
    ) -> Result<Arc<Session>> {
        let user_key = format!("{}:{}:{}", user_id, channel_id, chat_id);

        // Check for existing session
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
        self.user_sessions.write().await.insert(user_key, session_id);

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

    /// Terminate a session
    pub async fn terminate_session(&self, session_id: &str) -> Result<()> {
        let session = match self.sessions.write().await.remove(session_id) {
            Some(s) => s,
            None => return Ok(()),
        };

        session.set_state(SessionState::Terminating).await;

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
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let session = Session::new(
            "user-123".to_string(),
            "telegram".to_string(),
            "chat-456".to_string(),
        );

        assert_eq!(session.state().await, SessionState::Creating);
        assert!(!session.uses_tee);
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

        session.update_sensitivity(SensitivityLevel::Sensitive).await;
        assert_eq!(session.sensitivity_level().await, SensitivityLevel::Sensitive);

        // Should not decrease
        session.update_sensitivity(SensitivityLevel::Normal).await;
        assert_eq!(session.sensitivity_level().await, SensitivityLevel::Sensitive);
    }

    #[tokio::test]
    async fn test_session_manager() {
        let manager = SessionManager::new();

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
    async fn test_session_termination() {
        let manager = SessionManager::new();

        let session = manager
            .create_session("user-123", "telegram", "chat-456")
            .await
            .unwrap();
        let session_id = session.id.clone();

        manager.terminate_session(&session_id).await.unwrap();

        assert!(manager.get_session(&session_id).await.is_none());
        assert_eq!(manager.session_count().await, 0);
    }
}
