//! NDJSON protocol types for Claude Code CLI communication
//!
//! Defines all message types exchanged between:
//! - CLI ↔ Server (NDJSON, newline-delimited JSON)
//! - Server ↔ Browser (JSON)
//! - Session state and process info types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// CLI → Server messages (NDJSON)
// =============================================================================

/// Top-level CLI message (parsed from NDJSON lines)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CliMessage {
    System(CliSystemMessage),
    Assistant(CliAssistantMessage),
    Result(CliResultMessage),
    StreamEvent(CliStreamEventMessage),
    ControlRequest(CliControlRequestMessage),
    ToolProgress(CliToolProgressMessage),
    ToolUseSummary(CliToolUseSummaryMessage),
    AuthStatus(CliAuthStatusMessage),
    KeepAlive,
}

/// System message (subtype: "init" or "status")
#[derive(Debug, Clone, Deserialize)]
pub struct CliSystemMessage {
    pub subtype: String,
    pub uuid: Option<String>,
    pub session_id: Option<String>,
    // init fields
    pub cwd: Option<String>,
    pub tools: Option<Vec<String>>,
    pub model: Option<String>,
    #[serde(rename = "permissionMode")]
    pub permission_mode: Option<String>,
    pub claude_code_version: Option<String>,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub agents: Option<Vec<String>>,
    pub slash_commands: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    // status fields
    pub status: Option<String>,
}

/// MCP server info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub name: String,
    pub status: String,
}

/// Assistant message (complete model response)
#[derive(Debug, Clone, Deserialize)]
pub struct CliAssistantMessage {
    pub message: AssistantMessageBody,
    pub parent_tool_use_id: Option<String>,
    pub uuid: Option<String>,
    pub session_id: Option<String>,
}

/// Body of an assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageBody {
    pub id: String,
    #[serde(rename = "type", default)]
    pub msg_type: Option<String>,
    pub role: String,
    pub model: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub usage: Option<serde_json::Value>,
}

/// Content block within an assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: serde_json::Value,
        #[serde(default)]
        is_error: bool,
    },
    Thinking {
        thinking: String,
        budget_tokens: Option<u32>,
    },
}

/// Result message (turn completion)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliResultMessage {
    pub subtype: String,
    pub is_error: bool,
    pub result: Option<String>,
    pub errors: Option<Vec<String>>,
    pub duration_ms: Option<u64>,
    pub num_turns: Option<u32>,
    pub total_cost_usd: Option<f64>,
    pub stop_reason: Option<String>,
    #[serde(rename = "modelUsage")]
    pub model_usage: Option<serde_json::Value>,
    pub total_lines_added: Option<u32>,
    pub total_lines_removed: Option<u32>,
    pub uuid: Option<String>,
    pub session_id: Option<String>,
}

/// Stream event (raw Anthropic SSE)
#[derive(Debug, Clone, Deserialize)]
pub struct CliStreamEventMessage {
    pub event: serde_json::Value,
    pub parent_tool_use_id: Option<String>,
}

/// Control request (permission prompt from CLI)
#[derive(Debug, Clone, Deserialize)]
pub struct CliControlRequestMessage {
    pub request_id: String,
    pub request: ControlRequestBody,
}

/// Body of a control request
#[derive(Debug, Clone, Deserialize)]
pub struct ControlRequestBody {
    pub subtype: String,
    pub tool_name: Option<String>,
    pub input: Option<serde_json::Value>,
    pub permission_suggestions: Option<Vec<serde_json::Value>>,
    pub description: Option<String>,
    pub tool_use_id: Option<String>,
    pub agent_id: Option<String>,
}

/// Tool progress heartbeat
#[derive(Debug, Clone, Deserialize)]
pub struct CliToolProgressMessage {
    pub tool_use_id: String,
    pub tool_name: String,
    pub parent_tool_use_id: Option<String>,
    pub elapsed_time_seconds: f64,
}

/// Tool use summary
#[derive(Debug, Clone, Deserialize)]
pub struct CliToolUseSummaryMessage {
    pub summary: String,
    pub preceding_tool_use_ids: Vec<String>,
}

/// Auth status
#[derive(Debug, Clone, Deserialize)]
pub struct CliAuthStatusMessage {
    #[serde(rename = "isAuthenticating")]
    pub is_authenticating: bool,
    pub output: Vec<String>,
    pub error: Option<String>,
}

// =============================================================================
// Server → Browser messages (JSON)
// =============================================================================

/// Message sent to browser clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrowserIncomingMessage {
    SessionInit {
        session: AgentSessionState,
    },
    SessionUpdate {
        session: serde_json::Value,
    },
    Assistant {
        message: AssistantMessageBody,
        parent_tool_use_id: Option<String>,
    },
    StreamEvent {
        event: serde_json::Value,
        parent_tool_use_id: Option<String>,
    },
    Result {
        data: serde_json::Value,
    },
    PermissionRequest {
        request: PermissionRequest,
    },
    PermissionCancelled {
        request_id: String,
    },
    ToolProgress {
        tool_use_id: String,
        tool_name: String,
        elapsed_time_seconds: f64,
    },
    ToolUseSummary {
        summary: String,
        tool_use_ids: Vec<String>,
    },
    StatusChange {
        status: Option<String>,
    },
    AuthStatus {
        is_authenticating: bool,
        output: Vec<String>,
        error: Option<String>,
    },
    Error {
        message: String,
    },
    CliConnected,
    CliDisconnected,
    UserMessage {
        content: String,
        timestamp: u64,
    },
    MessageHistory {
        messages: Vec<BrowserIncomingMessage>,
    },
    SessionNameUpdate {
        name: String,
    },
}

// =============================================================================
// Browser → Server messages (JSON)
// =============================================================================

/// Message received from browser clients
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrowserOutgoingMessage {
    UserMessage {
        content: String,
        session_id: Option<String>,
        images: Option<Vec<ImageAttachment>>,
    },
    PermissionResponse {
        request_id: String,
        behavior: String,
        updated_input: Option<serde_json::Value>,
        updated_permissions: Option<Vec<serde_json::Value>>,
        message: Option<String>,
    },
    Interrupt,
    SetModel {
        model: String,
    },
    SetPermissionMode {
        mode: String,
    },
}

/// Base64 image attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAttachment {
    pub media_type: String,
    pub data: String,
}

// =============================================================================
// Session state types
// =============================================================================

/// Agent session state (populated from CLI system.init)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionState {
    pub session_id: String,
    pub model: String,
    pub cwd: String,
    pub tools: Vec<String>,
    pub permission_mode: String,
    pub claude_code_version: String,
    pub mcp_servers: Vec<McpServer>,
    pub agents: Vec<String>,
    pub slash_commands: Vec<String>,
    pub skills: Vec<String>,
    pub total_cost_usd: f64,
    pub num_turns: u32,
    pub context_used_percent: f64,
    pub is_compacting: bool,
    pub total_lines_added: u32,
    pub total_lines_removed: u32,
}

impl AgentSessionState {
    /// Create a new empty session state
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            model: String::new(),
            cwd: String::new(),
            tools: Vec::new(),
            permission_mode: "default".to_string(),
            claude_code_version: String::new(),
            mcp_servers: Vec::new(),
            agents: Vec::new(),
            slash_commands: Vec::new(),
            skills: Vec::new(),
            total_cost_usd: 0.0,
            num_turns: 0,
            context_used_percent: 0.0,
            is_compacting: false,
            total_lines_added: 0,
            total_lines_removed: 0,
        }
    }
}

/// Agent process info (metadata about a CLI process)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProcessInfo {
    pub session_id: String,
    pub pid: Option<u32>,
    pub state: AgentProcessState,
    pub exit_code: Option<i32>,
    pub model: Option<String>,
    pub permission_mode: Option<String>,
    pub cwd: String,
    pub created_at: u64,
    /// CLI's internal session ID (used for --resume)
    pub cli_session_id: Option<String>,
    pub archived: bool,
    pub name: Option<String>,
}

/// Agent process lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentProcessState {
    /// Process spawned, waiting for WS connection
    Starting,
    /// WS connection established
    Connected,
    /// Actively processing messages
    Running,
    /// Process terminated
    Exited,
}

/// Permission request from CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub request_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
    pub permission_suggestions: Option<Vec<serde_json::Value>>,
    pub description: Option<String>,
    pub tool_use_id: Option<String>,
    pub agent_id: Option<String>,
    pub timestamp: u64,
}

/// Persisted agent session (for disk storage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedAgentSession {
    pub id: String,
    pub state: AgentSessionState,
    pub message_history: Vec<BrowserIncomingMessage>,
    pub pending_messages: Vec<String>,
    pub pending_permissions: HashMap<String, PermissionRequest>,
    pub archived: bool,
}

// =============================================================================
// Helper functions
// =============================================================================

/// Parse NDJSON data into CLI messages, skipping malformed lines
pub fn parse_ndjson(data: &str) -> Vec<CliMessage> {
    data.split('\n')
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            serde_json::from_str::<CliMessage>(line)
                .map_err(|e| {
                    let preview = &line[..line.len().min(200)];
                    tracing::warn!("NDJSON parse error: {} (line: {})", e, preview);
                })
                .ok()
        })
        .collect()
}

/// Compute context usage percentage from modelUsage
pub fn compute_context_percent(model_usage: &serde_json::Value) -> f64 {
    if let Some(obj) = model_usage.as_object() {
        for (_model, usage) in obj {
            let input = usage
                .get("inputTokens")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let output = usage
                .get("outputTokens")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let context_window = usage
                .get("contextWindow")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);

            if context_window > 0.0 {
                let percent = (input + output) / context_window * 100.0;
                return percent.clamp(0.0, 100.0);
            }
        }
    }
    0.0
}

/// Convert a browser user_message to CLI NDJSON format
pub fn user_message_to_cli_ndjson(
    content: &str,
    images: &Option<Vec<ImageAttachment>>,
    cli_session_id: &Option<String>,
) -> serde_json::Value {
    let message_content = if let Some(imgs) = images {
        if !imgs.is_empty() {
            let mut blocks: Vec<serde_json::Value> = imgs
                .iter()
                .map(|img| {
                    serde_json::json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": img.media_type,
                            "data": img.data,
                        }
                    })
                })
                .collect();
            blocks.push(serde_json::json!({
                "type": "text",
                "text": content,
            }));
            serde_json::Value::Array(blocks)
        } else {
            serde_json::Value::String(content.to_string())
        }
    } else {
        serde_json::Value::String(content.to_string())
    };

    serde_json::json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": message_content,
        },
        "parent_tool_use_id": null,
        "session_id": cli_session_id,
    })
}

/// Convert a permission allow response to CLI NDJSON format
pub fn permission_allow_to_cli_ndjson(
    request_id: &str,
    updated_input: &Option<serde_json::Value>,
    updated_permissions: &Option<Vec<serde_json::Value>>,
) -> serde_json::Value {
    let mut response = serde_json::json!({
        "behavior": "allow",
    });
    if let Some(input) = updated_input {
        response["updatedInput"] = input.clone();
    }
    if let Some(perms) = updated_permissions {
        response["updatedPermissions"] = serde_json::Value::Array(perms.clone());
    }

    serde_json::json!({
        "type": "control_response",
        "response": {
            "subtype": "success",
            "request_id": request_id,
            "response": response,
        }
    })
}

/// Convert a permission deny response to CLI NDJSON format
pub fn permission_deny_to_cli_ndjson(
    request_id: &str,
    message: &Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "type": "control_response",
        "response": {
            "subtype": "success",
            "request_id": request_id,
            "response": {
                "behavior": "deny",
                "message": message.as_deref().unwrap_or("Denied by user"),
            }
        }
    })
}

/// Create an interrupt control request for CLI
pub fn interrupt_to_cli_ndjson() -> serde_json::Value {
    serde_json::json!({
        "type": "control_request",
        "request_id": uuid::Uuid::new_v4().to_string(),
        "request": {
            "subtype": "interrupt",
        }
    })
}

/// Create a set_model control request for CLI
pub fn set_model_to_cli_ndjson(model: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "control_request",
        "request_id": uuid::Uuid::new_v4().to_string(),
        "request": {
            "subtype": "set_model",
            "model": model,
        }
    })
}

/// Create a set_permission_mode control request for CLI
pub fn set_permission_mode_to_cli_ndjson(mode: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "control_request",
        "request_id": uuid::Uuid::new_v4().to_string(),
        "request": {
            "subtype": "set_permission_mode",
            "mode": mode,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_system_init_message() {
        let line = r#"{"type":"system","subtype":"init","uuid":"abc","session_id":"s1","cwd":"/tmp","tools":["Bash","Read"],"model":"claude-sonnet-4-20250514","permissionMode":"default","claude_code_version":"1.0.0","mcp_servers":[],"agents":[],"slash_commands":[],"skills":[]}"#;
        let msgs = parse_ndjson(line);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            CliMessage::System(sys) => {
                assert_eq!(sys.subtype, "init");
                assert_eq!(sys.model.as_deref(), Some("claude-sonnet-4-20250514"));
                assert_eq!(sys.tools.as_ref().unwrap().len(), 2);
            }
            _ => panic!("Expected System message"),
        }
    }

    #[test]
    fn test_parse_assistant_message() {
        let line = r#"{"type":"assistant","message":{"id":"msg1","role":"assistant","model":"claude-sonnet-4-20250514","content":[{"type":"text","text":"Hello"}],"stop_reason":"end_turn"},"parent_tool_use_id":null,"uuid":"u1","session_id":"s1"}"#;
        let msgs = parse_ndjson(line);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            CliMessage::Assistant(ast) => {
                assert_eq!(ast.message.content.len(), 1);
                assert!(ast.parent_tool_use_id.is_none());
            }
            _ => panic!("Expected Assistant message"),
        }
    }

    #[test]
    fn test_parse_control_request() {
        let line = r#"{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"ls"},"tool_use_id":"tu1"}}"#;
        let msgs = parse_ndjson(line);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            CliMessage::ControlRequest(cr) => {
                assert_eq!(cr.request_id, "r1");
                assert_eq!(cr.request.tool_name.as_deref(), Some("Bash"));
            }
            _ => panic!("Expected ControlRequest message"),
        }
    }

    #[test]
    fn test_parse_result_message() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"num_turns":1,"total_cost_usd":0.01,"duration_ms":500,"stop_reason":"end_turn","uuid":"u1","session_id":"s1"}"#;
        let msgs = parse_ndjson(line);
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            CliMessage::Result(res) => {
                assert!(!res.is_error);
                assert_eq!(res.total_cost_usd, Some(0.01));
            }
            _ => panic!("Expected Result message"),
        }
    }

    #[test]
    fn test_parse_stream_event() {
        let line = r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hi"}},"parent_tool_use_id":null}"#;
        let msgs = parse_ndjson(line);
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], CliMessage::StreamEvent(_)));
    }

    #[test]
    fn test_parse_keep_alive() {
        let line = r#"{"type":"keep_alive"}"#;
        let msgs = parse_ndjson(line);
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], CliMessage::KeepAlive));
    }

    #[test]
    fn test_parse_ndjson_multiple_lines() {
        let data = r#"{"type":"keep_alive"}
{"type":"keep_alive"}
{"type":"keep_alive"}"#;
        let msgs = parse_ndjson(data);
        assert_eq!(msgs.len(), 3);
    }

    #[test]
    fn test_parse_ndjson_skips_malformed() {
        let data = "not json\n{\"type\":\"keep_alive\"}\n{broken";
        let msgs = parse_ndjson(data);
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], CliMessage::KeepAlive));
    }

    #[test]
    fn test_compute_context_percent() {
        let usage = serde_json::json!({
            "claude-sonnet-4-20250514": {
                "inputTokens": 8000,
                "outputTokens": 2000,
                "contextWindow": 200000,
            }
        });
        let pct = compute_context_percent(&usage);
        assert!((pct - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_context_percent_clamps() {
        let usage = serde_json::json!({
            "model": {
                "inputTokens": 200000,
                "outputTokens": 100000,
                "contextWindow": 200000,
            }
        });
        let pct = compute_context_percent(&usage);
        assert_eq!(pct, 100.0);
    }

    #[test]
    fn test_user_message_to_cli_ndjson_text_only() {
        let ndjson = user_message_to_cli_ndjson("hello", &None, &Some("s1".to_string()));
        assert_eq!(ndjson["type"], "user");
        assert_eq!(ndjson["message"]["content"], "hello");
        assert_eq!(ndjson["session_id"], "s1");
    }

    #[test]
    fn test_user_message_to_cli_ndjson_with_images() {
        let images = vec![ImageAttachment {
            media_type: "image/png".to_string(),
            data: "base64data".to_string(),
        }];
        let ndjson = user_message_to_cli_ndjson("describe this", &Some(images), &None);
        let content = &ndjson["message"]["content"];
        assert!(content.is_array());
        let arr = content.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["type"], "image");
        assert_eq!(arr[1]["type"], "text");
    }

    #[test]
    fn test_permission_allow_to_cli_ndjson() {
        let ndjson = permission_allow_to_cli_ndjson("r1", &None, &None);
        assert_eq!(ndjson["type"], "control_response");
        assert_eq!(ndjson["response"]["response"]["behavior"], "allow");
        assert_eq!(ndjson["response"]["request_id"], "r1");
    }

    #[test]
    fn test_permission_deny_to_cli_ndjson() {
        let ndjson = permission_deny_to_cli_ndjson("r1", &Some("no".to_string()));
        assert_eq!(ndjson["response"]["response"]["behavior"], "deny");
        assert_eq!(ndjson["response"]["response"]["message"], "no");
    }

    #[test]
    fn test_interrupt_to_cli_ndjson() {
        let ndjson = interrupt_to_cli_ndjson();
        assert_eq!(ndjson["type"], "control_request");
        assert_eq!(ndjson["request"]["subtype"], "interrupt");
        assert!(ndjson["request_id"].is_string());
    }

    #[test]
    fn test_set_model_to_cli_ndjson() {
        let ndjson = set_model_to_cli_ndjson("claude-opus-4-20250514");
        assert_eq!(ndjson["request"]["subtype"], "set_model");
        assert_eq!(ndjson["request"]["model"], "claude-opus-4-20250514");
    }

    #[test]
    fn test_set_permission_mode_to_cli_ndjson() {
        let ndjson = set_permission_mode_to_cli_ndjson("plan");
        assert_eq!(ndjson["request"]["subtype"], "set_permission_mode");
        assert_eq!(ndjson["request"]["mode"], "plan");
    }

    #[test]
    fn test_browser_incoming_message_serialization() {
        let msg = BrowserIncomingMessage::CliConnected;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("cli_connected"));

        let msg = BrowserIncomingMessage::Error {
            message: "test error".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("test error"));
    }

    #[test]
    fn test_browser_outgoing_message_deserialization() {
        let json = r#"{"type":"user_message","content":"hello"}"#;
        let msg: BrowserOutgoingMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, BrowserOutgoingMessage::UserMessage { .. }));

        let json = r#"{"type":"interrupt"}"#;
        let msg: BrowserOutgoingMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, BrowserOutgoingMessage::Interrupt));
    }

    #[test]
    fn test_agent_session_state_new() {
        let state = AgentSessionState::new("test-id".to_string());
        assert_eq!(state.session_id, "test-id");
        assert_eq!(state.total_cost_usd, 0.0);
        assert_eq!(state.num_turns, 0);
        assert!(!state.is_compacting);
    }

    #[test]
    fn test_content_block_serialization() {
        let block = ContentBlock::ToolUse {
            id: "tu1".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("tool_use"));
        assert!(json.contains("Bash"));

        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, ContentBlock::ToolUse { .. }));
    }
}
