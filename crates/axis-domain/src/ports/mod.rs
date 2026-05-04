pub type StatusStream<T> = std::pin::Pin<Box<dyn futures_util::Stream<Item = T> + Send>>;

use async_trait::async_trait;

#[async_trait]
pub trait StatusProvider<S>: Send + Sync {
    type Error: std::error::Error + Send + 'static;
    async fn get_status(&self) -> Result<S, Self::Error>;
    async fn subscribe(&self) -> Result<StatusStream<S>, Self::Error>;
}

#[macro_export]
macro_rules! status_provider {
    ($provider_trait:ident, $status:ty, $error:ty) => {
        #[async_trait]
        impl<T: $provider_trait + ?Sized> $crate::ports::StatusProvider<$status> for T {
            type Error = $error;
            async fn get_status(&self) -> Result<$status, Self::Error> {
                $provider_trait::get_status(self).await
            }
            async fn subscribe(&self) -> Result<$crate::ports::StatusStream<$status>, Self::Error> {
                $provider_trait::subscribe(self).await
            }
        }
    };
}

pub mod airplane;
pub mod ssh;
pub mod appearance;
pub mod agenda;
pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod calendar;
pub mod clock;
pub mod config;
pub mod continuity;
pub mod dnd;
pub mod idle_inhibit;
pub mod mpris;
pub mod network;
pub mod nightlight;
pub mod notifications;
pub mod power;
pub mod tasks;
pub mod cloud;
pub mod cloud_auth;
pub mod workspaces;
pub mod layout;
pub mod popups;
pub mod launcher;
pub mod ipc;
pub mod lock;
pub mod tray;
