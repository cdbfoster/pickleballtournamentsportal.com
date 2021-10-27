use std::future::Future;
use std::sync::{LockResult, Mutex, RwLock, RwLockReadGuard};
use std::time::{Duration, Instant};

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

    // Not perfect; it's possible two threads will want to update at the same time and clobber each other.  Oh well.
    // We can't hold the timestamp mutex and remain async during the call to update at the same time.
    pub async fn retrieve_or_update<F, E>(
        &self,
        interval: Duration,
        update: impl Fn() -> F,
    ) -> Result<LockResult<RwLockReadGuard<'_, T>>, E>
    where
        F: Future<Output = Result<T, E>>,
    {
        let should_update = {
            let timestamp = self.timestamp.lock().unwrap();
            timestamp.is_none() || timestamp.unwrap().elapsed() >= interval
        };

        if should_update {
            match update().await {
                Ok(new_value) => {
                    *self.value.write().unwrap() = new_value;
                    *self.timestamp.lock().unwrap() = Some(Instant::now());
                }
                Err(error) => return Err(error),
            }
        }

        Ok(self.value.read())
    }
}
