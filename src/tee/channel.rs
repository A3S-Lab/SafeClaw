//! RA-TLS communication channel to the TEE guest.
//!
//! Provides HTTP-over-RA-TLS communication with the guest's attestation server.
//! Each request opens a new RA-TLS connection (the guest server is connection-per-request).
//!
//! Endpoints:
//! - `GET /status` — TEE status check
//! - `POST /secrets` — Secret injection (handled by `SecretInjector`)
//! - `POST /seal` — Seal data (handled by `SealClient`)
//! - `POST /unseal` — Unseal data (handled by `SealClient`)
//! - `POST /process` — Process a message through the TEE-resident agent

use crate::error::{Error, Result};

use a3s_box_runtime::tee::ratls::create_client_config;
use a3s_box_runtime::tee::AttestationPolicy;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

/// RA-TLS communication channel to the TEE guest.
///
/// Wraps the attestation socket path and policy, providing typed methods
/// for each guest endpoint. Every call performs a fresh RA-TLS handshake,
/// which re-verifies the TEE attestation.
#[derive(Debug, Clone)]
pub struct RaTlsChannel {
    socket_path: PathBuf,
    policy: AttestationPolicy,
    allow_simulated: bool,
}

/// Response from the guest's `POST /process` endpoint.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProcessResponse {
    pub session_id: String,
    pub content: String,
    pub success: bool,
    #[serde(default)]
    pub error: Option<String>,
}

/// Request payload for the guest's `POST /process` endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessRequest {
    pub session_id: String,
    pub content: String,
    pub request_type: String,
}

impl RaTlsChannel {
    /// Create a new RA-TLS channel.
    pub fn new(socket_path: &Path, policy: AttestationPolicy, allow_simulated: bool) -> Self {
        Self {
            socket_path: socket_path.to_path_buf(),
            policy,
            allow_simulated,
        }
    }

    /// Get the socket path.
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Check TEE status via `GET /status`.
    pub async fn status(&self) -> Result<serde_json::Value> {
        let body = self
            .request("GET", "/status", None)
            .await?;
        serde_json::from_str(&body).map_err(|e| {
            Error::Tee(format!("Invalid status response: {}", e))
        })
    }

    /// Process a message through the TEE-resident agent via `POST /process`.
    pub async fn process(
        &self,
        session_id: &str,
        content: &str,
        request_type: &str,
    ) -> Result<ProcessResponse> {
        let req = ProcessRequest {
            session_id: session_id.to_string(),
            content: content.to_string(),
            request_type: request_type.to_string(),
        };

        let payload = serde_json::to_string(&req).map_err(|e| {
            Error::Tee(format!("Failed to serialize process request: {}", e))
        })?;

        let body = self
            .request("POST", "/process", Some(&payload))
            .await?;

        let resp: ProcessResponse = serde_json::from_str(&body).map_err(|e| {
            Error::Tee(format!("Invalid process response: {}", e))
        })?;

        if !resp.success {
            return Err(Error::Tee(format!(
                "TEE processing failed: {}",
                resp.error.as_deref().unwrap_or("unknown error")
            )));
        }

        Ok(resp)
    }

    /// Send an HTTP request over RA-TLS and return the response body.
    ///
    /// Each call opens a new TLS connection. The RA-TLS handshake verifies
    /// the TEE attestation before any data is exchanged.
    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Result<String> {
        // Build RA-TLS client config
        let client_config =
            create_client_config(self.policy.clone(), self.allow_simulated).map_err(|e| {
                Error::Tee(format!("Failed to create RA-TLS config: {}", e))
            })?;

        let connector =
            tokio_rustls::TlsConnector::from(Arc::new(client_config));

        // Connect to the Unix socket
        let stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            Error::Tee(format!(
                "Failed to connect to TEE at {}: {}",
                self.socket_path.display(),
                e
            ))
        })?;

        // TLS handshake — attestation is verified here
        let server_name = rustls::pki_types::ServerName::try_from("localhost")
            .map_err(|e| Error::Tee(format!("Invalid server name: {}", e)))?;

        let mut tls_stream = connector.connect(server_name, stream).await.map_err(|e| {
            Error::Tee(format!("RA-TLS handshake failed: {}", e))
        })?;

        // Build HTTP request
        let http_request = if let Some(payload) = body {
            format!(
                "{} {} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                method, path, payload.len(), payload
            )
        } else {
            format!(
                "{} {} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                method, path
            )
        };

        // Send request
        tls_stream
            .write_all(http_request.as_bytes())
            .await
            .map_err(|e| Error::Tee(format!("RA-TLS write failed: {}", e)))?;

        // Read response (handle peer closing without close_notify)
        let mut response = Vec::with_capacity(65536);
        match tls_stream.read_to_end(&mut response).await {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                tracing::debug!("RA-TLS peer closed without close_notify (harmless)");
            }
            Err(e) => {
                return Err(Error::Tee(format!("RA-TLS read failed: {}", e)));
            }
        }

        // Parse HTTP response — extract body after \r\n\r\n
        let response_str = String::from_utf8_lossy(&response);
        let body = response_str
            .find("\r\n\r\n")
            .map(|pos| response_str[pos + 4..].to_string())
            .unwrap_or_else(|| response_str.to_string());

        // Check HTTP status
        if response_str.starts_with("HTTP/1.1 4") || response_str.starts_with("HTTP/1.1 5") {
            return Err(Error::Tee(format!("TEE request failed: {}", body)));
        }

        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let channel = RaTlsChannel::new(
            Path::new("/tmp/test.sock"),
            AttestationPolicy::default(),
            true,
        );
        assert_eq!(channel.socket_path(), Path::new("/tmp/test.sock"));
        assert!(channel.allow_simulated);
    }

    #[test]
    fn test_channel_clone() {
        let channel = RaTlsChannel::new(
            Path::new("/tmp/test.sock"),
            AttestationPolicy::default(),
            false,
        );
        let cloned = channel.clone();
        assert_eq!(cloned.socket_path(), channel.socket_path());
        assert_eq!(cloned.allow_simulated, channel.allow_simulated);
    }

    #[test]
    fn test_process_request_serialization() {
        let req = ProcessRequest {
            session_id: "s1".to_string(),
            content: "hello".to_string(),
            request_type: "process_message".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"session_id\":\"s1\""));
        assert!(json.contains("\"content\":\"hello\""));
        assert!(json.contains("\"request_type\":\"process_message\""));
    }

    #[test]
    fn test_process_response_deserialization() {
        let json = r#"{"session_id":"s1","content":"response","success":true}"#;
        let resp: ProcessResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.content, "response");
        assert!(resp.success);
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_process_response_with_error() {
        let json = r#"{"session_id":"s1","content":"","success":false,"error":"agent down"}"#;
        let resp: ProcessResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.success);
        assert_eq!(resp.error.as_deref(), Some("agent down"));
    }
}
