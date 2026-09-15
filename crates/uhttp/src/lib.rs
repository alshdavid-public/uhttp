#![deny(unused_crate_dependencies)]

pub mod body;
#[cfg(feature = "file_server")]
pub mod file_server;
mod handler;
#[cfg(feature = "router")]
pub mod middleware;
mod request;
mod response;
mod response_body;
#[cfg(feature = "router")]
pub mod router;
mod server;
#[cfg(feature = "json")]
pub mod sse;
mod types;
#[cfg(feature = "websocket")]
mod websocket;

pub use http::StatusCode;
pub use tokio_util::sync::CancellationToken;

pub use self::handler::*;
pub use self::request::*;
pub use self::response::*;
pub use self::response_body::*;
#[cfg(feature = "router")]
pub use self::router::*;
pub use self::server::*;
#[cfg(feature = "websocket")]
pub use self::websocket::*;
