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
    pub async fn retrieve_or_update<F>(
        &self,
        interval: Duration,
        update: impl Fn() -> F,
    ) -> LockResult<RwLockReadGuard<'_, T>>
    where
        F: Future<Output = T>,
    {
        let should_update = {
            let timestamp = self.timestamp.lock().unwrap();
            timestamp.is_none() || timestamp.unwrap().elapsed() >= interval
        };

        if should_update {
            let new_value = update().await;
            *self.value.write().unwrap() = new_value;
            *self.timestamp.lock().unwrap() = Some(Instant::now());
        }

        self.value.read()
    }
}
