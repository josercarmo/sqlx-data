use ahash::{AHasher, RandomState};
use dashmap::DashMap;
use moka::sync::Cache;

use std::{
    hash::Hash,
    hash::Hasher,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub last_access: Instant,
}

pub struct GlobalCache<K, V> {
    cache: Cache<K, Arc<V>>,
    pub stats: DashMap<K, CacheStats, RandomState>,
}

impl<K, V> GlobalCache<K, V>
where
    K: Eq + Hash + Copy + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    pub fn new(
        initial_capacity: usize,
        max_capacity: u64,
        ttl: Option<Duration>,
        weigher: impl Fn(&K, &V) -> u32 + Send + Sync + 'static,
    ) -> Self {
        let mut builder = Cache::builder()
            .initial_capacity(initial_capacity)
            .max_capacity(max_capacity);

        if let Some(duration) = ttl {
            builder = builder.time_to_live(duration);
        }

        let cache = builder.weigher(move |k, v: &Arc<V>| weigher(k, v)).build();

        Self {
            cache,
            stats: DashMap::with_capacity_and_hasher(initial_capacity, RandomState::new()),
        }
    }

    /// Synchronous version, without race conditions
    #[allow(dead_code)]
    pub fn get_or_insert_with<F>(&self, key: K, builder: F) -> Arc<V>
    where
        F: FnOnce(&K) -> V,
    {
        let now = Instant::now();
        let mut was_miss = false;

        let value = self.cache.get_with(key, || {
            was_miss = true;
            self.record_miss(key, now);
            Arc::new(builder(&key))
        });

        if !was_miss {
            self.record_hit(key, now);
        }

        value
    }

    /// Directly inserts an already validated value into the cache
    pub fn insert(&self, key: K, value: V) -> Arc<V> {
        let arc_value = Arc::new(value);
        let now = Instant::now();

        self.cache.insert(key, arc_value.clone());
        self.record_hit(key, now);

        arc_value
    }

    /// Searches for a value in the cache
    pub fn get(&self, key: K) -> Option<Arc<V>> {
        let now = Instant::now();

        if let Some(value) = self.cache.get(&key) {
            self.record_hit(key, now);
            Some(value)
        } else {
            self.record_miss(key, now);
            None
        }
    }

    fn record_hit(&self, key: K, now: Instant) {
        self.stats
            .entry(key)
            .and_modify(|s| {
                s.hits += 1;
                s.last_access = now;
            })
            .or_insert(CacheStats {
                hits: 1,
                misses: 0,
                last_access: now,
            });
    }

    fn record_miss(&self, key: K, now: Instant) {
        self.stats
            .entry(key)
            .and_modify(|s| {
                s.misses += 1;
                s.last_access = now;
            })
            .or_insert(CacheStats {
                hits: 0,
                misses: 1,
                last_access: now,
            });
    }

    #[allow(dead_code)]
    pub fn stats(&self, key: &K) -> Option<CacheStats> {
        self.stats.get(key).map(|s| s.clone())
    }

    #[allow(dead_code)]
    pub fn invalidate(&self, key: &K) {
        self.cache.invalidate(key);
        self.stats.remove(key);
    }

    #[allow(dead_code)]
    pub fn clear(&self) {
        self.cache.invalidate_all();
        self.stats.clear();
    }

    pub fn fingerprint(&self, value: &str) -> u64 {
        let mut hasher = AHasher::default();
        hasher.write(value.as_bytes());
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {}
