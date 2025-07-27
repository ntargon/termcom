//! TermCom Library
//! 
//! Embedded device communication debug tool library providing
//! serial and TCP communication capabilities with session management.

pub mod cli;
pub mod tui;
pub mod core;
pub mod domain;
pub mod infrastructure;

pub use domain::error::{TermComError, TermComResult};
pub use domain::config::TermComConfig;
pub use core::session::{Session, SessionManager, SessionType, SessionId};
pub use core::communication::CommunicationEngine;