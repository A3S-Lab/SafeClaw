//! Privacy classification and data protection
//!
//! Provides automatic detection of sensitive data and routing
//! decisions for TEE processing. Includes:
//! - Regex-based classification (PII patterns)
//! - Semantic analysis (natural language PII disclosure)
//! - Compliance rule engines (HIPAA, PCI-DSS, GDPR)

mod classifier;
pub mod compliance;
mod policy;
pub mod semantic;

pub use classifier::{ClassificationResult, Classifier, Match};
pub use compliance::{ComplianceEngine, ComplianceFramework, ComplianceRuleSet};
pub use policy::{DataPolicy, PolicyBuilder, PolicyDecision, PolicyEngine};
pub use semantic::{SemanticAnalyzer, SemanticCategory, SemanticMatch};
