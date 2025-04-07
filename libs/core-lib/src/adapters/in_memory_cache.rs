use crate::{Cache, CoreError};
use async_trait::async_trait;
use moka::future::Cache as MokaCache;
use std::time::Duration;

/// In-memory implementation of the Cache port using Moka.
/// Suitable for testing and single-executable mode.
#[derive(Clone, Debug)]
pub struct InMemoryCache {
    cache: MokaCache<String, Vec<u8>>,
}

impl InMemoryCache {
    /// Creates a new InMemoryCache with specific capacity and default TTL settings.
    pub fn new(max_capacity: u64, default_ttl_seconds: u64) -> Self {
        let cache = MokaCache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(default_ttl_seconds))
            // Consider adding time_to_idle as well if needed
            .build();
        Self { cache }
    }
}

impl Default for InMemoryCache {
    /// Creates a new InMemoryCache with default capacity (e.g., 10,000) and TTL (e.g., 1 hour).
    fn default() -> Self {
        Self::new(10_000, 3600)
    }
}

#[async_trait]
impl Cache for InMemoryCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CoreError> {
        Ok(self.cache.get(key).await)
    }

    async fn set(
        &self,
        key: &str,
        value: &[u8],
        _ttl_seconds: Option<u64>, // Prefixed with underscore as it's unused in this adapter
    ) -> Result<(), CoreError> {
        // Moka's async cache sets TTL at build time.
        // The `insert` method respects the pre-configured TTL.
        // We ignore the `ttl_seconds` parameter for this specific adapter.
        self.cache.insert(key.to_string(), value.to_vec()).await;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), CoreError> {
        self.cache.invalidate(key).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_set_and_get() {
        let cache = InMemoryCache::default();
        let key = "test_key";
        let value = b"test_value".to_vec();

        cache.set(key, &value, None).await.unwrap();
        let retrieved = cache.get(key).await.unwrap();

        assert_eq!(retrieved, Some(value));
    }

    #[tokio::test]
    async fn test_get_non_existent() {
        let cache = InMemoryCache::default();
        let key = "non_existent_key";

        let retrieved = cache.get(key).await.unwrap();
        assert_eq!(retrieved, None);
    }

    #[tokio::test]
    async fn test_delete() {
        let cache = InMemoryCache::default();
        let key = "delete_key";
        let value = b"delete_value".to_vec();

        cache.set(key, &value, None).await.unwrap();
        let retrieved_before = cache.get(key).await.unwrap();
        assert_eq!(retrieved_before, Some(value));

        cache.delete(key).await.unwrap();
        let retrieved_after = cache.get(key).await.unwrap();
        assert_eq!(retrieved_after, None);
    }

    #[tokio::test]
    // Renamed test to reflect behavior: uses default TTL set during construction
    async fn test_entry_expires_based_on_default_ttl() {
        // Use a small capacity and TTL for testing expiry
        let default_ttl_seconds = 1;
        let cache = InMemoryCache::new(100, default_ttl_seconds); // 1 second default TTL
        let key = "ttl_key";
        let value = b"ttl_value".to_vec();

        // Set the value (ttl_seconds parameter is ignored by this adapter)
        cache.set(key, &value, None).await.unwrap();
        let retrieved_before = cache.get(key).await.unwrap();
        assert_eq!(
            retrieved_before,
            Some(value.clone()),
            "Value should be present immediately after set"
        );

        // Wait for slightly longer than the default TTL
        sleep(Duration::from_millis(1100)).await;

        let retrieved_after = cache.get(key).await.unwrap();
        assert_eq!(
            retrieved_after, None,
            "Cache entry should have expired based on default TTL"
        );
    }

    // Removed the redundant test_set_with_default_ttl as the previous test now covers this.

    #[tokio::test]
    async fn test_overwrite() {
        let cache = InMemoryCache::default();
        let key = "overwrite_key";
        let value1 = b"value1".to_vec();
        let value2 = b"value2".to_vec();

        cache.set(key, &value1, None).await.unwrap();
        let retrieved1 = cache.get(key).await.unwrap();
        assert_eq!(retrieved1, Some(value1));

        cache.set(key, &value2, None).await.unwrap();
        let retrieved2 = cache.get(key).await.unwrap();
        assert_eq!(retrieved2, Some(value2));
    }
}
