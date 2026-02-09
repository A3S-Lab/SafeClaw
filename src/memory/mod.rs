//! Memory system — three-layer data hierarchy
//!
//! Layer 1 (Resource): Raw classified content with storage routing.
//! Layer 2 (Artifact): Structured knowledge extracted from Resources.

pub mod artifact;
pub mod artifact_store;
pub mod extractor;
pub mod gate;
pub mod resource;
pub mod store;

pub use artifact::{Artifact, ArtifactType};
pub use artifact_store::ArtifactStore;
pub use extractor::Extractor;
pub use gate::PrivacyGate;
pub use resource::{ContentType, Resource, StorageLocation};
pub use store::ResourceStore;
