//! Memory system — three-layer data hierarchy
//!
//! Layer 1 (Resource): Raw classified content with storage routing.
//! Layer 2 (Artifact): Structured knowledge extracted from Resources.
//! Layer 3 (Insight): Cross-conversation knowledge synthesis from Artifacts.

pub mod artifact;
pub mod artifact_store;
pub mod extractor;
pub mod gate;
pub mod insight;
pub mod insight_store;
pub mod resource;
pub mod store;
pub mod synthesizer;

pub use artifact::{Artifact, ArtifactType};
pub use artifact_store::ArtifactStore;
pub use extractor::Extractor;
pub use gate::PrivacyGate;
pub use insight::{Insight, InsightType};
pub use insight_store::InsightStore;
pub use resource::{ContentType, Resource, StorageLocation};
pub use store::ResourceStore;
pub use synthesizer::Synthesizer;
