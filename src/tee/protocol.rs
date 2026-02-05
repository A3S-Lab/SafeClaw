//! TEE communication protocol

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message types for TEE communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TeeMessage {
    /// Request message
    Request(TeeRequest),
    /// Response message
    Response(TeeResponse),
    /// Heartbeat
    Heartbeat { timestamp: i64 },
    /// Error message
    Error { code: i32, message: String },
}

/// Request to TEE environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeeRequest {
    /// Unique request ID
    pub id: String,
    /// Session ID
    pub session_id: String,
    /// Request type
    pub request_type: TeeRequestType,
    /// Encrypted payload (when channel is established)
    pub payload: Vec<u8>,
    /// Timestamp
    pub timestamp: i64,
}

impl TeeRequest {
    /// Create a new request
    pub fn new(session_id: String, request_type: TeeRequestType, payload: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            request_type,
            payload,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// Types of TEE requests
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeeRequestType {
    /// Initialize a new session
    InitSession,
    /// Process a message with the AI agent
    ProcessMessage,
    /// Execute a tool
    ExecuteTool,
    /// Store sensitive data
    StoreSecret,
    /// Retrieve sensitive data
    RetrieveSecret,
    /// Delete sensitive data
    DeleteSecret,
    /// Get session state
    GetSessionState,
    /// Terminate session
    TerminateSession,
}

/// Response from TEE environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeeResponse {
    /// Request ID this responds to
    pub request_id: String,
    /// Session ID
    pub session_id: String,
    /// Response status
    pub status: TeeResponseStatus,
    /// Encrypted payload
    pub payload: Vec<u8>,
    /// Timestamp
    pub timestamp: i64,
}

impl TeeResponse {
    /// Create a success response
    pub fn success(request_id: String, session_id: String, payload: Vec<u8>) -> Self {
        Self {
            request_id,
            session_id,
            status: TeeResponseStatus::Success,
            payload,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Create an error response
    pub fn error(request_id: String, session_id: String, code: i32, message: String) -> Self {
        Self {
            request_id,
            session_id,
            status: TeeResponseStatus::Error { code, message },
            payload: Vec::new(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// Response status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeeResponseStatus {
    /// Request succeeded
    Success,
    /// Request failed
    Error { code: i32, message: String },
    /// Request is pending (async operation)
    Pending,
}

/// Payload for InitSession request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitSessionPayload {
    /// User identifier
    pub user_id: String,
    /// Channel identifier
    pub channel_id: String,
    /// Model configuration
    pub model_config: ModelConfigPayload,
    /// Session options
    pub options: SessionOptions,
}

/// Model configuration for TEE session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfigPayload {
    /// Provider name
    pub provider: String,
    /// Model name
    pub model: String,
    /// API key (encrypted)
    pub api_key_encrypted: Vec<u8>,
}

/// Session options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionOptions {
    /// Enable conversation history
    pub enable_history: bool,
    /// Maximum history length
    pub max_history: usize,
    /// Enable tool execution
    pub enable_tools: bool,
    /// Allowed tools
    pub allowed_tools: Vec<String>,
}

/// Payload for ProcessMessage request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMessagePayload {
    /// User message content
    pub content: String,
    /// Message role
    pub role: String,
    /// Attachments (encrypted)
    pub attachments: Vec<AttachmentPayload>,
}

/// Attachment in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentPayload {
    /// Attachment type
    pub attachment_type: String,
    /// Attachment data (encrypted)
    pub data: Vec<u8>,
    /// Metadata
    pub metadata: serde_json::Value,
}

/// Payload for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteToolPayload {
    /// Tool name
    pub tool_name: String,
    /// Tool arguments
    pub arguments: serde_json::Value,
    /// Execution context
    pub context: serde_json::Value,
}

/// Payload for secret storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSecretPayload {
    /// Secret key/name
    pub key: String,
    /// Secret value (encrypted)
    pub value: Vec<u8>,
    /// Expiration timestamp (optional)
    pub expires_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_creation() {
        let request = TeeRequest::new(
            "session-123".to_string(),
            TeeRequestType::ProcessMessage,
            vec![1, 2, 3],
        );

        assert!(!request.id.is_empty());
        assert_eq!(request.session_id, "session-123");
        assert!(request.timestamp > 0);
    }

    #[test]
    fn test_response_success() {
        let response = TeeResponse::success(
            "req-123".to_string(),
            "session-123".to_string(),
            vec![4, 5, 6],
        );

        assert_eq!(response.request_id, "req-123");
        assert!(matches!(response.status, TeeResponseStatus::Success));
    }

    #[test]
    fn test_response_error() {
        let response = TeeResponse::error(
            "req-123".to_string(),
            "session-123".to_string(),
            500,
            "Internal error".to_string(),
        );

        assert!(matches!(
            response.status,
            TeeResponseStatus::Error { code: 500, .. }
        ));
    }

    #[test]
    fn test_message_serialization() {
        let request = TeeRequest::new(
            "session-123".to_string(),
            TeeRequestType::InitSession,
            vec![],
        );
        let message = TeeMessage::Request(request);

        let json = serde_json::to_string(&message).unwrap();
        let parsed: TeeMessage = serde_json::from_str(&json).unwrap();

        assert!(matches!(parsed, TeeMessage::Request(_)));
    }
}
