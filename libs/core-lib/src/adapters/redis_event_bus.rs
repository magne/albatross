use crate::{CoreError, EventPublisher, EventSubscriber};
use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, aio::PubSub, AsyncCommands, Client};
use tracing::info;

// TODO: Define configuration struct for connection details, channel prefix, etc.

/// Redis implementation of EventPublisher and EventSubscriber using Pub/Sub.
#[derive(Clone)]
pub struct RedisEventBus {
    publish_connection: MultiplexedConnection,
    redis_url: String,
    channel_prefix: String,
}

impl RedisEventBus {
    #[allow(dead_code)]
    pub async fn new(redis_url: &str, channel_prefix: Option<&str>) -> Result<Self, CoreError> {
        let client = Client::open(redis_url)
            .map_err(|e| CoreError::Configuration(format!("Invalid Redis URL: {}", e)))?;
        let publish_connection = client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;
        info!("Redis Event Bus publisher connected.");
        Ok(Self {
            publish_connection,
            redis_url: redis_url.to_string(),
            channel_prefix: channel_prefix.unwrap_or("").to_string(),
        })
    }

    /// Creates a new PubSub connection for subscribing.
    #[allow(dead_code)]
    // Return the concrete async PubSub type from redis::aio
    pub async fn create_subscriber_connection(&self) -> Result<PubSub, CoreError> {
        let client = Client::open(self.redis_url.as_str())
            .map_err(|e| CoreError::Configuration(format!("Invalid Redis URL: {}", e)))?;
        client
            .get_async_pubsub()
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))
    }

    fn get_channel_name(&self, topic: &str) -> String {
        format!("{}{}", self.channel_prefix, topic)
    }
}

#[async_trait]
impl EventPublisher for RedisEventBus {
    async fn publish(
        &self,
        topic: &str,
        _event_type: &str,
        event_payload: &[u8],
    ) -> Result<(), CoreError> {
        let mut conn = self.publish_connection.clone();
        let channel = self.get_channel_name(topic);
        let _: usize = conn
            .publish(&channel, event_payload)
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;
        Ok(())
    }
}

#[async_trait]
impl EventSubscriber for RedisEventBus {}

// --- Integration Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers::runners::AsyncRunner;
    use testcontainers::ContainerAsync;
    use testcontainers_modules::redis::Redis as RedisImage;
    use tokio::time::{timeout, Duration};
    use tokio_stream::StreamExt;

    async fn setup_redis_pubsub() -> (
        RedisEventBus,
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
        let bus = RedisEventBus::new(&redis_url, Some("test_prefix:"))
            .await
            .expect("Failed to create RedisEventBus");
        (bus, node, redis_url)
    }

    #[tokio::test]
    async fn test_publish_and_subscribe_redis() {
        let (bus, _node, _url) = setup_redis_pubsub().await;
        let topic = "test-topic-redis";
        let event_type = "TestEventRedis";
        let payload = b"hello redis pubsub".to_vec();
        let channel_name = bus.get_channel_name(topic);

        // Setup subscriber connection (returns redis::aio::PubSub)
        let mut pubsub_conn = bus
            .create_subscriber_connection()
            .await
            .expect("Failed to create pubsub conn");
        // subscribe is async on PubSub
        pubsub_conn
            .subscribe(&channel_name)
            .await
            .expect("Failed to subscribe");

        // Get the message stream using into_on_message()
        let mut message_stream = pubsub_conn.into_on_message();

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(1);

        // Spawn a task to listen to the stream
        tokio::spawn(async move {
            if let Some(msg) = message_stream.next().await {
                let received_payload: Vec<u8> = msg.get_payload().expect("Failed to get payload");
                tx.send(received_payload)
                    .await
                    .expect("Failed to send payload back to test");
            } else {
                eprintln!("Message stream ended unexpectedly");
            }
        });

        tokio::time::sleep(Duration::from_millis(100)).await; // Give subscriber time

        let publish_result = bus.publish(topic, event_type, &payload).await;
        assert!(
            publish_result.is_ok(),
            "Publish failed: {:?}",
            publish_result.err()
        );

        match timeout(Duration::from_secs(5), rx.recv()).await {
            Ok(Some(received_payload)) => assert_eq!(received_payload, payload),
            Ok(None) => panic!("Consumer channel closed unexpectedly"),
            Err(_) => panic!("Test timed out waiting for message"),
        }
    }
}
