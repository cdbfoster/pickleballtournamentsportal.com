use std::collections::HashMap;
use std::future::Future;
use std::time::{Duration, Instant};

use async_std::sync::{Arc, Mutex, RwLock, RwLockReadGuard};
use reqwest::{Error, IntoUrl, Response, Url};

use crate::util::scrape_result::{scrape_result, ScrapeResult};

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

pub struct PageCache {
    cache: Mutex<HashMap<Url, Arc<Cache<String>>>>,
}

impl PageCache {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get<U>(&self, url: U) -> CachedPage
    where
        U: IntoUrl,
    {
        let mut cache = self.cache.lock().await;

        let url = url.into_url().unwrap();
        let page = cache
            .entry(url.clone())
            .or_insert(Arc::new(Cache::new()))
            .clone();

        CachedPage {
            cache: page,
            url: url.clone(),
        }
    }
}

pub struct CachedPage {
    cache: Arc<Cache<String>>,
    url: Url,
}

impl CachedPage {
    pub async fn retrieve_or_update<F>(
        &self,
        interval: Duration,
        fetch_url: impl Fn(Url) -> F,
        error: &str,
    ) -> ScrapeResult<RwLockReadGuard<'_, String>>
    where
        F: Future<Output = Result<Response, Error>>,
    {
        self.cache
            .retrieve_or_update(interval, || async {
                let response = scrape_result(fetch_url(self.url.clone()).await, error);
                match response {
                    Ok(response) => Ok(response.text().await.unwrap()),
                    Err(error) => Err(error),
                }
            })
            .await
    }
}
