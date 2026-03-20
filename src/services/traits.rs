use async_channel::{Receiver, Sender};

/// Unified service trait — every service implements this.
/// Read-only services use `type Cmd = ()`.
pub trait Service: 'static {
    type Data: Clone + PartialEq + Send + 'static;
    type Cmd: Send + 'static;

    fn spawn() -> (Receiver<Self::Data>, Sender<Self::Cmd>);
}
