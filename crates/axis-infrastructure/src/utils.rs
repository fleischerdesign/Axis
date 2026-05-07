use std::future::Future;
use std::time::Duration;

pub async fn retry_with_backoff<F, Fut, T, E>(mut f: F, max_delay_secs: u64) -> T
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut attempt = 0u64;
    loop {
        match f().await {
            Ok(value) => return value,
            Err(_) => {
                let delay = 2u64.pow(((attempt + 1).min(4)) as u32).min(max_delay_secs);
                tokio::time::sleep(Duration::from_secs(delay)).await;
                attempt += 1;
            }
        }
    }
}

pub fn retry_with_backoff_blocking<F, T, E>(mut f: F, max_delay_secs: u64) -> T
where
    F: FnMut() -> Result<T, E>,
{
    let mut attempt = 0u64;
    loop {
        match f() {
            Ok(value) => return value,
            Err(_) => {
                let delay = 2u64.pow(((attempt + 1).min(4)) as u32).min(max_delay_secs);
                std::thread::sleep(Duration::from_secs(delay));
                attempt += 1;
            }
        }
    }
}
