//! TEE session and lifecycle management

use super::client::TeeClient;
use crate::config::TeeConfig;
use crate::error::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// TEE session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeeSessionState {
    /// Session is being created
    Creating,
    /// Session is active and ready
    Active,
    /// Session is processing a request
    Busy,
    /// Session is being terminated
    Terminating,
    /// Session has been terminated
    Terminated,
}

/// A session running in the TEE environment
pub struct TeeSession {
    /// Session ID
    pub id: String,
    /// User ID associated with this session
    pub user_id: String,
    /// Channel ID (e.g., telegram, slack)
    pub channel_id: String,
    /// Current state
    state: Arc<RwLock<TeeSessionState>>,
    /// Creation timestamp
    pub created_at: i64,
    /// Last activity timestamp
    last_activity: Arc<RwLock<i64>>,
}

impl TeeSession {
    /// Create a new TEE session
    pub fn new(user_id: String, channel_id: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            channel_id,
            state: Arc::new(RwLock::new(TeeSessionState::Creating)),
            created_at: now,
            last_activity: Arc::new(RwLock::new(now)),
        }
    }

    /// Get current state
    pub async fn state(&self) -> TeeSessionState {
        *self.state.read().await
    }

    /// Set state
    pub async fn set_state(&self, state: TeeSessionState) {
        *self.state.write().await = state;
    }

    /// Update last activity timestamp
    pub async fn touch(&self) {
        *self.last_activity.write().await = chrono::Utc::now().timestamp_millis();
    }

    /// Get last activity timestamp
    pub async fn last_activity(&self) -> i64 {
        *self.last_activity.read().await
    }

    /// Check if session is active
    pub async fn is_active(&self) -> bool {
        matches!(
            self.state().await,
            TeeSessionState::Active | TeeSessionState::Busy
        )
    }
}

/// Manager for TEE sessions and lifecycle
pub struct TeeManager {
    config: TeeConfig,
    client: Arc<TeeClient>,
    sessions: Arc<RwLock<HashMap<String, Arc<TeeSession>>>>,
    /// Sessions indexed by user_id + channel_id
    user_sessions: Arc<RwLock<HashMap<String, String>>>,
}

impl TeeManager {
    /// Create a new TEE manager
    pub fn new(config: TeeConfig) -> Self {
        let client = Arc::new(TeeClient::new(config.clone()));
        Self {
            config,
            client,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            user_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the TEE manager
    pub async fn init(&self) -> Result<()> {
        if !self.config.enabled {
            tracing::info!("TEE is disabled, skipping initialization");
            return Ok(());
        }

        tracing::info!("Initializing TEE manager");
        self.client.connect().await?;
        tracing::info!("TEE manager initialized successfully");

        Ok(())
    }

    /// Shutdown the TEE manager
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down TEE manager");

        // Terminate all sessions
        let sessions: Vec<Arc<TeeSession>> = {
            let sessions = self.sessions.read().await;
            sessions.values().cloned().collect()
        };

        for session in sessions {
            if let Err(e) = self.terminate_session(&session.id).await {
                tracing::warn!("Failed to terminate session {}: {}", session.id, e);
            }
        }

        self.client.disconnect().await?;
        tracing::info!("TEE manager shutdown complete");

        Ok(())
    }

    /// Create a new TEE session
    pub async fn create_session(&self, user_id: &str, channel_id: &str) -> Result<Arc<TeeSession>> {
        if !self.config.enabled {
            return Err(Error::Tee("TEE is not enabled".to_string()));
        }

        // Check for existing session
        let user_key = format!("{}:{}", user_id, channel_id);
        if let Some(session_id) = self.user_sessions.read().await.get(&user_key) {
            if let Some(session) = self.sessions.read().await.get(session_id) {
                if session.is_active().await {
                    return Ok(session.clone());
                }
            }
        }

        // Create new session
        let session = Arc::new(TeeSession::new(user_id.to_string(), channel_id.to_string()));
        let session_id = session.id.clone();

        // Initialize in TEE
        self.client.init_session(&session_id, user_id).await?;

        // Update state
        session.set_state(TeeSessionState::Active).await;

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
            "Created TEE session {} for user {} on channel {}",
            session.id,
            user_id,
            channel_id
        );

        Ok(session)
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<TeeSession>> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Get session for a user on a channel
    pub async fn get_user_session(
        &self,
        user_id: &str,
        channel_id: &str,
    ) -> Option<Arc<TeeSession>> {
        let user_key = format!("{}:{}", user_id, channel_id);
        let session_id = self.user_sessions.read().await.get(&user_key)?.clone();
        self.get_session(&session_id).await
    }

    /// Process a message in a TEE session
    pub async fn process_message(&self, session_id: &str, content: &str) -> Result<String> {
        let session = self
            .get_session(session_id)
            .await
            .ok_or_else(|| Error::Tee(format!("Session {} not found", session_id)))?;

        if !session.is_active().await {
            return Err(Error::Tee("Session is not active".to_string()));
        }

        session.set_state(TeeSessionState::Busy).await;
        session.touch().await;

        let result = self.client.process_message(session_id, content).await;

        session.set_state(TeeSessionState::Active).await;

        result
    }

    /// Terminate a session
    pub async fn terminate_session(&self, session_id: &str) -> Result<()> {
        let session = match self.sessions.write().await.remove(session_id) {
            Some(s) => s,
            None => return Ok(()), // Already removed
        };

        session.set_state(TeeSessionState::Terminating).await;

        // Remove from user sessions
        let user_key = format!("{}:{}", session.user_id, session.channel_id);
        self.user_sessions.write().await.remove(&user_key);

        // Terminate in TEE
        self.client.terminate_session(session_id).await?;

        session.set_state(TeeSessionState::Terminated).await;

        tracing::info!("Terminated TEE session {}", session_id);

        Ok(())
    }

    /// Get all active sessions
    pub async fn active_sessions(&self) -> Vec<Arc<TeeSession>> {
        let sessions = self.sessions.read().await;
        let mut active = Vec::new();

        for session in sessions.values() {
            if session.is_active().await {
                active.push(session.clone());
            }
        }

        active
    }

    /// Clean up inactive sessions
    pub async fn cleanup_inactive(&self, max_idle_ms: i64) -> Result<usize> {
        let now = chrono::Utc::now().timestamp_millis();
        let sessions: Vec<Arc<TeeSession>> = {
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
            tracing::info!("Cleaned up {} inactive TEE sessions", cleaned);
        }

        Ok(cleaned)
    }

    /// Check if TEE is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get TEE client reference
    pub fn client(&self) -> &Arc<TeeClient> {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_lifecycle() {
        let session = TeeSession::new("user-123".to_string(), "telegram".to_string());

        assert_eq!(session.state().await, TeeSessionState::Creating);

        session.set_state(TeeSessionState::Active).await;
        assert!(session.is_active().await);

        session.set_state(TeeSessionState::Terminated).await;
        assert!(!session.is_active().await);
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let config = TeeConfig::default();
        let manager = TeeManager::new(config);

        assert!(manager.is_enabled());
    }

    #[tokio::test]
    async fn test_session_touch() {
        let session = TeeSession::new("user-123".to_string(), "telegram".to_string());
        let initial = session.last_activity().await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        session.touch().await;

        assert!(session.last_activity().await > initial);
    }
}
