//! Memory system — three-layer data hierarchy
//!
//! Layer 1 (Resource): Raw classified content with storage routing.

pub mod gate;
pub mod resource;
pub mod store;

pub use gate::PrivacyGate;
pub use resource::{ContentType, Resource, StorageLocation};
pub use store::ResourceStore;
