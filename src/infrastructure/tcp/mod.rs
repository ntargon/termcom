// TCP module - TCP communication implementation
pub mod client;
pub mod manager;
pub mod server;

pub use manager::TcpManager;
pub use server::EchoServer;