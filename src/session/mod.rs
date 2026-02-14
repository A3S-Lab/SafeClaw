//! Session management and routing

mod manager;
mod router;

pub use manager::{Session, SessionManager, SessionState};
#[cfg(feature = "mock-tee")]
pub use manager::TeeHandle;
pub use router::{RoutingDecision, SessionRouter};
