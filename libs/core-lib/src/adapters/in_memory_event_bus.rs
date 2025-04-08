use crate::{CoreError, EventPublisher, EventSubscriber};
use async_trait::async_trait;
use dashmap::DashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::broadcast::{self, Sender};

/// Represents a message published on the in-memory bus.
#[derive(Clone, Debug)]
pub struct InMemoryMessage {
    pub topic: String,
    pub event_type: String,
    pub payload: Vec<u8>,
}

/// In-memory implementation of the EventPublisher and EventSubscriber ports using Tokio broadcast channels.
/// Suitable for testing and single-executable mode.
///
/// Note: Tokio broadcast channels are multi-producer, multi-consumer, but if a receiver
/// lags behind, it might miss messages. This might be acceptable for some testing scenarios
/// but differs from persistent queues like RabbitMQ.
#[derive(Debug, Clone)]
pub struct InMemoryEventBus {
    // Store: Topic Name -> Broadcast Sender
    // We only store the Sender; Receivers are created on demand for subscribers.
    channels: Arc<DashMap<String, Sender<InMemoryMessage>>>,
    // Capacity for each new broadcast channel created.
    channel_capacity: usize,
}

impl InMemoryEventBus {
    /// Creates a new InMemoryEventBus with a specific capacity for broadcast channels.
    pub fn new(channel_capacity: usize) -> Self {
        Self {
            channels: Arc::new(DashMap::new()),
            channel_capacity,
        }
    }

    /// Gets or creates a broadcast sender for a given topic.
    fn get_or_create_sender(&self, topic: &str) -> Sender<InMemoryMessage> {
        self.channels
            .entry(topic.to_string())
            .or_insert_with(|| {
                let (sender, _) = broadcast::channel(self.channel_capacity);
                sender
            })
            .value()
            .clone()
    }

    /// Subscribes to a specific topic, returning a broadcast receiver.
    /// This is intended for internal use by adapters or test setups that need direct subscription.
    pub fn subscribe(&self, topic: &str) -> broadcast::Receiver<InMemoryMessage> {
        self.get_or_create_sender(topic).subscribe()
    }
}

impl Default for InMemoryEventBus {
    /// Creates a new InMemoryEventBus with a default channel capacity (e.g., 100).
    fn default() -> Self {
        Self::new(100)
    }
}

#[async_trait]
impl EventPublisher for InMemoryEventBus {
    async fn publish(
        &self,
        topic: &str,
        event_type: &str,
        event_payload: &[u8],
    ) -> Result<(), CoreError> {
        let sender = self.get_or_create_sender(topic);
        let message = InMemoryMessage {
            topic: topic.to_string(),
            event_type: event_type.to_string(),
            payload: event_payload.to_vec(),
        };

        // Sending on a broadcast channel returns the number of *active* subscribers.
        // An error occurs only if there are *no* active subscribers.
        // In many pub/sub scenarios (like event sourcing), publishing to a topic
        // with no current subscribers is not an error.
        match sender.send(message) {
            Ok(_) => Ok(()),
            Err(_) => {
                // Log this situation? It means no one is listening *right now*.
                // For an event bus, this is usually okay.
                // println!("Warning: Published to topic '{}' with no active subscribers.", topic);
                Ok(())
            }
        }
    }
}

// Basic implementation for the marker trait EventSubscriber.
// The actual subscription logic happens within the components (like projection workers)
// that *use* this bus, by calling the `subscribe` method.
#[async_trait]
impl EventSubscriber for InMemoryEventBus {
    // This trait implementation doesn't actively *do* anything here.
    // Components needing to subscribe will hold an instance of InMemoryEventBus
    // and call its `subscribe(topic)` method to get a receiver.
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_publish_and_subscribe() {
        let bus = InMemoryEventBus::default();
        let topic = "test-topic";
        let event_type = "TestEvent";
        let payload = b"hello world".to_vec();

        // Subscribe *before* publishing
        let mut receiver1 = bus.subscribe(topic);
        let mut receiver2 = bus.subscribe(topic);

        let publish_result = bus.publish(topic, event_type, &payload).await;
        assert!(publish_result.is_ok());

        // Check receiver 1
        let recv_result1 = timeout(Duration::from_millis(100), receiver1.recv()).await;
        assert!(recv_result1.is_ok(), "Receiver 1 timed out");
        let message1 = recv_result1.unwrap().unwrap();
        assert_eq!(message1.topic, topic);
        assert_eq!(message1.event_type, event_type);
        assert_eq!(message1.payload, payload);

        // Check receiver 2
        let recv_result2 = timeout(Duration::from_millis(100), receiver2.recv()).await;
        assert!(recv_result2.is_ok(), "Receiver 2 timed out");
        let message2 = recv_result2.unwrap().unwrap();
        assert_eq!(message2.topic, topic);
        assert_eq!(message2.event_type, event_type);
        assert_eq!(message2.payload, payload);
    }

    #[tokio::test]
    async fn test_publish_with_no_subscribers() {
        let bus = InMemoryEventBus::default();
        let topic = "lonely-topic";
        let event_type = "LonelyEvent";
        let payload = b"echo?".to_vec();

        // Publish without any active subscribers
        let publish_result = bus.publish(topic, event_type, &payload).await;
        // This should succeed, even if no one is listening immediately.
        assert!(publish_result.is_ok());
    }

    #[tokio::test]
    async fn test_subscribe_after_publish() {
        let bus = InMemoryEventBus::default();
        let topic = "missed-topic";
        let event_type = "MissedEvent";
        let payload = b"too late".to_vec();

        // Publish first
        let publish_result = bus.publish(topic, event_type, &payload).await;
        assert!(publish_result.is_ok());

        // Subscribe *after* publishing
        let mut receiver = bus.subscribe(topic);

        // Attempt to receive - should timeout or return RecvError::Lagged initially
        // because broadcast doesn't guarantee delivery of past messages.
        let recv_result = timeout(Duration::from_millis(50), receiver.recv()).await;
        assert!(recv_result.is_err(), "Should have timed out or lagged");

        // Now publish again
        let payload2 = b"second message".to_vec();
        bus.publish(topic, event_type, &payload2).await.unwrap();

        // Should receive the second message
        let recv_result2 = timeout(Duration::from_millis(100), receiver.recv()).await;
        assert!(
            recv_result2.is_ok(),
            "Receiver timed out for second message"
        );
        let message2 = recv_result2.unwrap().unwrap();
        assert_eq!(message2.payload, payload2);
    }

    #[tokio::test]
    async fn test_multiple_topics() {
        let bus = InMemoryEventBus::default();
        let topic1 = "topic-1";
        let topic2 = "topic-2";
        let event_type = "MultiEvent";
        let payload1 = b"message 1".to_vec();
        let payload2 = b"message 2".to_vec();

        let mut receiver1 = bus.subscribe(topic1);
        let mut receiver2 = bus.subscribe(topic2);

        // Publish to topic 1
        bus.publish(topic1, event_type, &payload1).await.unwrap();

        // Publish to topic 2
        bus.publish(topic2, event_type, &payload2).await.unwrap();

        // Check receiver 1 gets message 1
        let recv_result1 = timeout(Duration::from_millis(100), receiver1.recv()).await;
        assert!(recv_result1.is_ok(), "Receiver 1 timed out");
        assert_eq!(recv_result1.unwrap().unwrap().payload, payload1);

        // Check receiver 2 gets message 2
        let recv_result2 = timeout(Duration::from_millis(100), receiver2.recv()).await;
        assert!(recv_result2.is_ok(), "Receiver 2 timed out");
        assert_eq!(recv_result2.unwrap().unwrap().payload, payload2);

        // Check receiver 1 doesn't get message 2 (should timeout)
        let recv_result1_again = timeout(Duration::from_millis(50), receiver1.recv()).await;
        assert!(
            recv_result1_again.is_err(),
            "Receiver 1 got message meant for topic 2"
        );
    }
}
