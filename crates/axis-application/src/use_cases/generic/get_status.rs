use axis_domain::ports::StatusProvider;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct GetStatusUseCase<P: ?Sized, S> {
    provider: Arc<P>,
    _phantom: PhantomData<S>,
}

impl<P, S> GetStatusUseCase<P, S>
where
    P: StatusProvider<S> + ?Sized,
{
    pub fn new(provider: Arc<P>) -> Self {
        Self {
            provider,
            _phantom: PhantomData,
        }
    }

    pub async fn execute(&self) -> Result<S, P::Error> {
        self.provider.get_status().await
    }
}
