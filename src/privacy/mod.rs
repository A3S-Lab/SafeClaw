//! Privacy classification and data protection
//!
//! Provides automatic detection of sensitive data and routing
//! decisions for TEE processing. Includes:
//! - Regex-based classification (PII patterns)
//! - Semantic analysis (natural language PII disclosure)
//! - Pluggable classifier backend architecture
//! - Compliance rule engines (HIPAA, PCI-DSS, GDPR)

pub mod backend;
pub mod classifier;
pub mod compliance;
pub mod cumulative;
pub mod handler;
mod policy;
pub mod semantic;

pub use backend::{
    ClassifierBackend, CompositeClassifier, CompositeResult, LlmBackend, LlmClassifierFn,
    PiiMatch, RegexBackend, SemanticBackend,
};
pub use classifier::{ClassificationResult, Classifier, Match};
pub use compliance::{ComplianceEngine, ComplianceFramework, ComplianceRuleSet};
pub use cumulative::{CumulativeRiskDecision, PiiType, SessionPrivacyContext};
pub use handler::{privacy_router, PrivacyState};
pub use policy::{DataPolicy, PolicyBuilder, PolicyDecision, PolicyEngine};
pub use semantic::{SemanticAnalyzer, SemanticCategory, SemanticMatch};
