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

pub mod alerting;
pub mod audit;
pub mod bus;
pub mod firewall;
pub mod handler;
pub mod injection;
pub mod interceptor;
pub mod isolation;
pub mod sanitizer;
pub mod taint;

pub use alerting::{Alert, AlertConfig, AlertKind, AlertMonitor};
pub use audit::{AuditEvent, AuditLog, AuditSeverity, LeakageVector};
pub use bus::AuditEventBus;
pub use firewall::{FirewallDecision, FirewallResult, NetworkFirewall, NetworkPolicy};
pub use handler::{audit_router, AuditState};
pub use injection::{InjectionCategory, InjectionDetector, InjectionResult, InjectionVerdict};
pub use interceptor::{InterceptDecision, InterceptResult, ToolInterceptor};
pub use isolation::{SessionIsolation, WipeResult};
pub use sanitizer::{OutputSanitizer, SanitizeResult};
pub use taint::{TaintEntry, TaintMatch, TaintRegistry, TaintType};
