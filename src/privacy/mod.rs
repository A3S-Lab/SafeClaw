//! Privacy classification and data protection
//!
//! Provides automatic detection of sensitive data and routing
//! decisions for TEE processing.

mod classifier;
mod policy;

pub use classifier::{Classifier, ClassificationResult, Match};
pub use policy::{DataPolicy, PolicyDecision, PolicyEngine};
