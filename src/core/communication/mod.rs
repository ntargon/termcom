// Communication module - Communication engine abstraction
pub mod engine;
pub mod message;
pub mod transport;

pub use engine::CommunicationEngine;
pub use message::{Message, MessagePattern};
pub use transport::TransportType;