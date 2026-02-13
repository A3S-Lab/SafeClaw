//! AI Agent Leakage Prevention
//!
//! Prevents sensitive data from leaking through AI agent outputs,
//! tool calls, or encoded variants. Implements taint tracking,
//! output sanitization, tool call interception, and audit logging.
//!
//! ## Architecture
//!
//! ```text
//! Input → TaintRegistry (mark sensitive data)
//!              ↓
//! AI Agent processes message
//!              ↓
//! Output → OutputSanitizer (scan & redact tainted data)
//! Tool calls → ToolInterceptor (block tainted args & dangerous commands)
//!              ↓
//! AuditLog (record all blocked attempts)
//! ```

pub mod audit;
pub mod interceptor;
pub mod sanitizer;
pub mod taint;

pub use audit::{AuditEvent, AuditLog, AuditSeverity, LeakageVector};
pub use interceptor::{InterceptDecision, InterceptResult, ToolInterceptor};
pub use sanitizer::{OutputSanitizer, SanitizeResult};
pub use taint::{TaintEntry, TaintMatch, TaintRegistry, TaintType};
