//! TEE client for communicating with the secure environment

use super::protocol::{TeeMessage, TeeRequest, TeeRequestType, TeeResponse, TeeResponseStatus};
use crate::config::TeeConfig;
use crate::crypto::SecureChannel;
use crate::error::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};

/// Client for communicating with TEE environment
pub struct TeeClient {
    config: TeeConfig,
    secure_channel: Arc<SecureChannel>,
    pending_requests: Arc<RwLock<HashMap<String, oneshot::Sender<TeeResponse>>>>,
    connected: Arc<RwLock<bool>>,
}

impl TeeClient {
    /// Create a new TEE client
    pub fn new(config: TeeConfig) -> Self {
        let channel_id = format!("tee-client-{}", uuid::Uuid::new_v4());
        Self {
            config,
            secure_channel: Arc::new(SecureChannel::new(channel_id)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Check if connected to TEE
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Connect to the TEE environment
    pub async fn connect(&self) -> Result<()> {
        if !self.config.enabled {
            return Err(Error::Tee("TEE is not enabled".to_string()));
        }

        // Start handshake
        let _handshake_init = self.secure_channel.start_handshake().await?;

        // In a real implementation, this would:
        // 1. Connect to the A3S Box via vsock
        // 2. Exchange public keys
        // 3. Complete the handshake

        // For now, we simulate the connection
        tracing::info!(
            "TEE client connecting to {}:{}",
            self.config.box_image,
            self.config.vsock_port
        );

        *self.connected.write().await = true;

        Ok(())
    }

    /// Disconnect from TEE
    pub async fn disconnect(&self) -> Result<()> {
        self.secure_channel.close().await;
        *self.connected.write().await = false;
        Ok(())
    }

    /// Send a request to TEE and wait for response
    pub async fn send_request(&self, request: TeeRequest) -> Result<TeeResponse> {
        if !self.is_connected().await {
            return Err(Error::Tee("Not connected to TEE".to_string()));
        }

        let request_id = request.id.clone();

        // Create response channel
        let (tx, _rx) = oneshot::channel();
        self.pending_requests
            .write()
            .await
            .insert(request_id.clone(), tx);

        // Serialize and encrypt request
        let message = TeeMessage::Request(request);
        let _serialized = serde_json::to_vec(&message)
            .map_err(|e| Error::Tee(format!("Failed to serialize request: {}", e)))?;

        // In a real implementation, send via vsock
        // For now, simulate the response
        let response = self.simulate_tee_response(&request_id).await;

        // Send response through channel
        if let Some(tx) = self.pending_requests.write().await.remove(&request_id) {
            let _ = tx.send(response.clone());
        }

        Ok(response)
    }

    /// Initialize a session in TEE
    pub async fn init_session(&self, session_id: &str, user_id: &str) -> Result<()> {
        let payload = serde_json::json!({
            "user_id": user_id,
            "session_id": session_id,
        });

        let request = TeeRequest::new(
            session_id.to_string(),
            TeeRequestType::InitSession,
            serde_json::to_vec(&payload).unwrap_or_default(),
        );

        let response = self.send_request(request).await?;

        match response.status {
            TeeResponseStatus::Success => Ok(()),
            TeeResponseStatus::Error { code, message } => Err(Error::Tee(format!(
                "Init session failed: {} ({})",
                message, code
            ))),
            TeeResponseStatus::Pending => Ok(()), // Async init
        }
    }

    /// Process a message in TEE
    pub async fn process_message(&self, session_id: &str, content: &str) -> Result<String> {
        let payload = serde_json::json!({
            "content": content,
            "role": "user",
        });

        let request = TeeRequest::new(
            session_id.to_string(),
            TeeRequestType::ProcessMessage,
            serde_json::to_vec(&payload).unwrap_or_default(),
        );

        let response = self.send_request(request).await?;

        match response.status {
            TeeResponseStatus::Success => {
                let result: serde_json::Value = serde_json::from_slice(&response.payload)
                    .map_err(|e| Error::Tee(format!("Failed to parse response: {}", e)))?;
                Ok(result["content"].as_str().unwrap_or("").to_string())
            }
            TeeResponseStatus::Error { code, message } => Err(Error::Tee(format!(
                "Process message failed: {} ({})",
                message, code
            ))),
            TeeResponseStatus::Pending => Err(Error::Tee("Unexpected pending status".to_string())),
        }
    }

    /// Store a secret in TEE
    pub async fn store_secret(&self, session_id: &str, key: &str, value: &[u8]) -> Result<()> {
        let payload = serde_json::json!({
            "key": key,
            "value": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, value),
        });

        let request = TeeRequest::new(
            session_id.to_string(),
            TeeRequestType::StoreSecret,
            serde_json::to_vec(&payload).unwrap_or_default(),
        );

        let response = self.send_request(request).await?;

        match response.status {
            TeeResponseStatus::Success => Ok(()),
            TeeResponseStatus::Error { code, message } => Err(Error::Tee(format!(
                "Store secret failed: {} ({})",
                message, code
            ))),
            _ => Ok(()),
        }
    }

    /// Retrieve a secret from TEE
    pub async fn retrieve_secret(&self, session_id: &str, key: &str) -> Result<Vec<u8>> {
        let payload = serde_json::json!({
            "key": key,
        });

        let request = TeeRequest::new(
            session_id.to_string(),
            TeeRequestType::RetrieveSecret,
            serde_json::to_vec(&payload).unwrap_or_default(),
        );

        let response = self.send_request(request).await?;

        match response.status {
            TeeResponseStatus::Success => {
                let result: serde_json::Value = serde_json::from_slice(&response.payload)
                    .map_err(|e| Error::Tee(format!("Failed to parse response: {}", e)))?;
                let encoded = result["value"].as_str().unwrap_or("");
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
                    .map_err(|e| Error::Tee(format!("Failed to decode secret: {}", e)))
            }
            TeeResponseStatus::Error { code, message } => Err(Error::Tee(format!(
                "Retrieve secret failed: {} ({})",
                message, code
            ))),
            _ => Err(Error::Tee("Unexpected response status".to_string())),
        }
    }

    /// Terminate a session in TEE
    pub async fn terminate_session(&self, session_id: &str) -> Result<()> {
        let request = TeeRequest::new(
            session_id.to_string(),
            TeeRequestType::TerminateSession,
            Vec::new(),
        );

        let response = self.send_request(request).await?;

        match response.status {
            TeeResponseStatus::Success => Ok(()),
            TeeResponseStatus::Error { code, message } => Err(Error::Tee(format!(
                "Terminate session failed: {} ({})",
                message, code
            ))),
            _ => Ok(()),
        }
    }

    /// Simulate TEE response (for development/testing)
    async fn simulate_tee_response(&self, request_id: &str) -> TeeResponse {
        // In production, this would be replaced with actual vsock communication
        TeeResponse::success(
            request_id.to_string(),
            "simulated-session".to_string(),
            serde_json::to_vec(&serde_json::json!({
                "content": "Response from TEE environment",
                "status": "ok"
            }))
            .unwrap_or_default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TeeConfig;

    #[tokio::test]
    async fn test_client_creation() {
        let config = TeeConfig::default();
        let client = TeeClient::new(config);

        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_connect_disconnect() {
        let config = TeeConfig::default();
        let client = TeeClient::new(config);

        client.connect().await.unwrap();
        assert!(client.is_connected().await);

        client.disconnect().await.unwrap();
        assert!(!client.is_connected().await);
    }
}
