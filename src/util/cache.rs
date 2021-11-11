use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::time::{Duration, Instant};

use async_std::sync::{Mutex, RwLock, RwLockReadGuard};
use reqwest::{Error, IntoUrl, Response, Url};

use crate::scrape::{scrape_result, ScrapeResult};
use crate::util::guard_stack::GuardStack;

pub type CacheGuard<'a, T> = RwLockReadGuard<'a, T>;

/// Retrieves the stored value unless the storage interval has expired in which case the value is updated
#[derive(Default)]
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
    ) -> Result<CacheGuard<'_, T>, E>
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

pub type CacheMapGuard<'a, K, V> = RwLockReadGuard<'a, HashMap<K, Cache<V>>>;

#[derive(Default)]
pub struct CacheMap<K, V> {
    cache_lock: Mutex<()>,
    cache: RwLock<HashMap<K, Cache<V>>>,
}

impl<K, V> CacheMap<K, V>
where
    K: Eq + Hash,
    V: Default,
{
    pub fn new() -> Self {
        Self {
            cache_lock: Mutex::new(()),
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get<'a>(&'a self, key: K) -> GuardStack<'a, CacheMapGuard<'a, K, V>, Cache<V>>
    where
        K: Clone,
    {
        let _cache_lock = self.cache_lock.lock().await;

        let cache_read = self.cache.read().await;

        if cache_read.contains_key(&key) {
            GuardStack::new(cache_read).map(|c| c.get(&key).unwrap())
        } else {
            std::mem::drop(cache_read);

            {
                let mut cache_write = self.cache.write().await;
                cache_write.insert(key.clone(), Cache::new());
            }

            GuardStack::new(self.cache.read().await).map(|c| c.get(&key).unwrap())
        }
    }
}

pub type PageCacheGuard<'a> =
    GuardStack<'a, (CacheMapGuard<'a, Url, String>, CacheGuard<'a, String>), String>;

pub struct PageCache(CacheMap<Url, String>);

impl PageCache {
    pub fn new() -> Self {
        Self(CacheMap::new())
    }

    pub async fn retrieve_or_update<'a, F, U>(
        &'a self,
        interval: Duration,
        url: U,
        fetch_url: impl Fn(Url) -> F,
        error: &str,
    ) -> ScrapeResult<PageCacheGuard<'a>>
    where
        F: Future<Output = Result<Response, Error>>,
        U: IntoUrl,
    {
        let url = url.into_url().unwrap();

        self.0
            .get(url.clone())
            .await
            .try_push_guard_async(|c| async move {
                c.retrieve_or_update(interval, || async {
                    let response = scrape_result(fetch_url(url.clone()).await, error);
                    match response {
                        Ok(response) => Ok(response.text().await.unwrap()),
                        Err(error) => Err(error),
                    }
                })
                .await
            })
            .await
    }
}

impl Default for PageCache {
    fn default() -> Self {
        Self::new()
    }
}
