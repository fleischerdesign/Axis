use crate::models::config::AxisConfig;
use futures_util::Stream;
use std::pin::Pin;

pub type ConfigStream = Pin<Box<dyn Stream<Item = AxisConfig> + Send>>;

pub trait ConfigProvider: Send + Sync {
    fn get(&self) -> AxisConfig;
    fn subscribe(&self) -> ConfigStream;
    fn update(&self, apply: Box<dyn FnOnce(&mut AxisConfig) + Send + 'static>);
}
