pub mod clipboard;
pub mod proto;
pub mod known_peers;
pub mod discovery;
pub mod connection;
pub mod input;
pub mod dbus;
pub mod proxy;
mod inner;
mod service;

pub use service::ContinuityService;
pub use proxy::ContinuityDbusProxy;
pub use inner::ContinuityCmd;
