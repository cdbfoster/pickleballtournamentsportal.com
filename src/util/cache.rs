use std::future::Future;
use std::time::{Duration, Instant};

use async_std::sync::{Mutex, RwLock, RwLockReadGuard};

/// Retrieves the stored value unless the storage interval has expired in which case the value is updated
pub struct Cache<T> {
    timestamp: Mutex<Option<Instant>>,
    value: RwLock<T>,
}

impl<T> Cache<T>
where
    T: Default,
{
    pub fn new() -> Self {
        Self {
            timestamp: Mutex::new(None),
            value: RwLock::new(T::default()),
        }
    }

    pub async fn retrieve_or_update<F, E>(
        &self,
        interval: Duration,
        update: impl Fn() -> F,
    ) -> Result<RwLockReadGuard<'_, T>, E>
    where
        F: Future<Output = Result<T, E>>,
    {
        let mut timestamp = self.timestamp.lock().await;

        if timestamp.is_none() || timestamp.unwrap().elapsed() >= interval {
            match update().await {
                Ok(new_value) => {
                    *self.value.write().await = new_value;
                    *timestamp = Some(Instant::now());
                }
                Err(error) => return Err(error),
            }
        }

        Ok(self.value.read().await)
    }
}
