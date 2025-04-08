use crate::{Cache, CoreError};
use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands, Client};
use tracing::info;

/// Redis implementation of the Cache port using redis-rs.
#[derive(Clone, Debug)]
pub struct RedisCache {
    connection: MultiplexedConnection,
    default_ttl_seconds: u64,
}

impl RedisCache {
    /// Creates a new RedisCache and connects to the server.
    #[allow(dead_code)]
    pub async fn new(redis_url: &str, default_ttl_seconds: u64) -> Result<Self, CoreError> {
        let client = Client::open(redis_url)
            .map_err(|e| CoreError::Configuration(format!("Invalid Redis URL: {}", e)))?;
        let connection = client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;
        info!("Redis Cache connected.");
        Ok(Self {
            connection,
            default_ttl_seconds,
        })
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CoreError> {
        let mut conn = self.connection.clone();
        conn.get(key)
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))
    }

    async fn set(
        &self,
        key: &str,
        value: &[u8],
        ttl_seconds: Option<u64>,
    ) -> Result<(), CoreError> {
        let mut conn = self.connection.clone();
        let ttl = ttl_seconds.unwrap_or(self.default_ttl_seconds);

        conn.set_ex(key, value, ttl)
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))
    }

    async fn delete(&self, key: &str) -> Result<(), CoreError> {
        let mut conn = self.connection.clone();
        // Explicitly ignore the usize result from DEL command
        conn.del(key)
            .await
            .map(|_: usize| ()) // Map Ok(usize) to Ok(())
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))
    }
}

// --- Integration Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    // Use modules crate for testcontainers image
    use redis::Client;
    use std::time::Duration;
    use testcontainers::runners::AsyncRunner;
    use testcontainers::ContainerAsync;
    use testcontainers_modules::redis::Redis as RedisImage;
    use tokio::time::sleep;

    async fn setup_redis() -> (
        MultiplexedConnection,
        ContainerAsync<RedisImage>,
        String, // Redis URL
    ) {
        let image = RedisImage::default();
        let node = image
            .start()
            .await
            .expect("Failed to start Redis container");
        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get host port");
        let redis_url = format!("redis://localhost:{}/", port);

        let client = Client::open(redis_url.clone()).expect("Failed to create test Redis client");
        let connection = client
            .get_multiplexed_tokio_connection()
            .await
            .expect("Failed to connect to testcontainer Redis");
        (connection, node, redis_url)
    }

    #[tokio::test]
    async fn test_set_and_get_redis() {
        let (conn, _node, _url) = setup_redis().await;
        let cache = RedisCache {
            connection: conn,
            default_ttl_seconds: 3600,
        };
        let key = "test_key_redis";
        let value = b"test_value_redis".to_vec();

        cache.set(key, &value, None).await.expect("SET failed");
        let retrieved = cache.get(key).await.expect("GET failed");
        assert_eq!(retrieved, Some(value));
    }

    #[tokio::test]
    async fn test_get_non_existent_redis() {
        let (conn, _node, _url) = setup_redis().await;
        let cache = RedisCache {
            connection: conn,
            default_ttl_seconds: 3600,
        };
        let key = "non_existent_key_redis";
        let retrieved = cache.get(key).await.expect("GET failed");
        assert_eq!(retrieved, None);
    }

    #[tokio::test]
    async fn test_delete_redis() {
        let (conn, _node, _url) = setup_redis().await;
        let cache = RedisCache {
            connection: conn,
            default_ttl_seconds: 3600,
        };
        let key = "delete_key_redis";
        let value = b"delete_value_redis".to_vec();

        cache.set(key, &value, None).await.expect("SET failed");
        let retrieved_before = cache.get(key).await.expect("GET before delete failed");
        assert_eq!(retrieved_before, Some(value));

        cache.delete(key).await.expect("DELETE failed"); // Should now return Ok(())
        let retrieved_after = cache.get(key).await.expect("GET after delete failed");
        assert_eq!(retrieved_after, None);
    }

    #[tokio::test]
    async fn test_set_with_ttl_redis() {
        let (conn, _node, _url) = setup_redis().await;
        let cache = RedisCache {
            connection: conn,
            default_ttl_seconds: 3600,
        };
        let key = "ttl_key_redis";
        let value = b"ttl_value_redis".to_vec();
        let ttl_seconds = 1;

        cache
            .set(key, &value, Some(ttl_seconds))
            .await
            .expect("SET with TTL failed");
        let retrieved_before = cache.get(key).await.expect("GET before expiry failed");
        assert_eq!(retrieved_before, Some(value.clone()));

        sleep(Duration::from_millis(1100)).await;
        let retrieved_after = cache.get(key).await.expect("GET after expiry failed");
        assert_eq!(retrieved_after, None, "Cache entry should have expired");
    }

    #[tokio::test]
    async fn test_set_with_default_ttl_redis() {
        let default_ttl = 1;
        let (conn, _node, _url) = setup_redis().await;
        let cache = RedisCache {
            connection: conn,
            default_ttl_seconds: default_ttl,
        };
        let key = "default_ttl_key_redis";
        let value = b"default_ttl_value_redis".to_vec();

        cache
            .set(key, &value, None)
            .await
            .expect("SET with default TTL failed");
        let retrieved_before = cache.get(key).await.expect("GET before expiry failed");
        assert_eq!(retrieved_before, Some(value.clone()));

        sleep(Duration::from_millis(1100)).await;
        let retrieved_after = cache.get(key).await.expect("GET after expiry failed");
        assert_eq!(
            retrieved_after, None,
            "Cache entry should have expired based on default TTL"
        );
    }
}
