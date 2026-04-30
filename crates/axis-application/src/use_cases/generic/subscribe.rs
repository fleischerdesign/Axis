use std::marker::PhantomData;
use std::sync::Arc;
use axis_domain::ports::{StatusProvider, StatusStream};

pub struct SubscribeUseCase<P: ?Sized, S> {
    provider: Arc<P>,
    _phantom: PhantomData<S>,
}

impl<P, S> SubscribeUseCase<P, S>
where
    P: StatusProvider<S> + ?Sized,
{
    pub fn new(provider: Arc<P>) -> Self {
        Self {
            provider,
            _phantom: PhantomData,
        }
    }

    pub async fn execute(&self) -> Result<StatusStream<S>, P::Error> {
        self.provider.subscribe().await
    }
}
