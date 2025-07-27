// Session module - Session management
pub mod manager;
pub mod session;
pub mod state;

pub use manager::{SessionManager, SessionFilter, SessionSummary};
pub use session::{Session, SessionConfig, SessionType};
pub use state::{SessionState, SessionStatus};

/// Session ID type alias
pub type SessionId = String;