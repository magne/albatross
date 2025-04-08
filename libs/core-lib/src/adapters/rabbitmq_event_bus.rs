use crate::{CoreError, EventPublisher, EventSubscriber};
use async_trait::async_trait;
use lapin::{
    options::{BasicPublishOptions, ExchangeDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind,
};
use std::sync::Arc;
use tracing::info;

// TODO: Define configuration struct for connection details, exchange name, etc.

/// RabbitMQ implementation of the EventPublisher and EventSubscriber ports using lapin.
#[derive(Clone)]
pub struct RabbitMqEventBus {
    connection: Arc<Connection>,
    publish_channel: Arc<Channel>,
    exchange_name: String,
}

impl RabbitMqEventBus {
    /// Creates a new RabbitMqEventBus and connects to the server.
    #[allow(dead_code)]
    pub async fn new(amqp_addr: &str, exchange_name: &str) -> Result<Self, CoreError> {
        let conn_options = ConnectionProperties::default();
        let connection = Connection::connect(amqp_addr, conn_options)
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;
        info!("RabbitMQ connected.");

        let publish_channel = connection
            .create_channel()
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;
        info!("RabbitMQ publish channel created.");

        publish_channel
            .exchange_declare(
                exchange_name,
                ExchangeKind::Topic,
                ExchangeDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;
        info!("RabbitMQ exchange '{}' declared.", exchange_name);

        Ok(Self {
            connection: Arc::new(connection),
            publish_channel: Arc::new(publish_channel),
            exchange_name: exchange_name.to_string(),
        })
    }

    /// Creates a new channel for subscribing.
    #[allow(dead_code)]
    pub async fn create_subscriber_channel(&self) -> Result<Channel, CoreError> {
        self.connection
            .create_channel()
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))
    }
}

#[async_trait]
impl EventPublisher for RabbitMqEventBus {
    async fn publish(
        &self,
        routing_key: &str,
        event_type: &str,
        event_payload: &[u8],
    ) -> Result<(), CoreError> {
        let properties = BasicProperties::default()
            .with_content_type("application/protobuf".into())
            .with_type(event_type.into())
            .with_delivery_mode(2);

        self.publish_channel
            .basic_publish(
                &self.exchange_name,
                routing_key,
                BasicPublishOptions::default(),
                event_payload,
                properties,
            )
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))?
            .await // Wait for confirmation
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;
        Ok(())
    }
}

#[async_trait]
impl EventSubscriber for RabbitMqEventBus {}

// --- Integration Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::Stream;
    use lapin::{
        message::Delivery,
        options::{BasicConsumeOptions, QueueBindOptions, QueueDeclareOptions},
        Channel, Connection, ConnectionProperties,
    };
    use testcontainers::runners::AsyncRunner;
    use testcontainers::ContainerAsync;
    use testcontainers_modules::rabbitmq::RabbitMq;
    use tokio::sync::mpsc;
    use tokio_stream::StreamExt;

    // Modified setup function to return the bus directly
    async fn setup_rabbitmq_bus(
        exchange_name: &str,
    ) -> (RabbitMqEventBus, ContainerAsync<RabbitMq>) {
        let image = RabbitMq::default();
        let node = image
            .start()
            .await
            .expect("Failed to start RabbitMQ container");
        let port = node
            .get_host_port_ipv4(5672)
            .await
            .expect("Failed to get host port");
        let amqp_addr = format!("amqp://guest:guest@localhost:{}", port);

        let conn_options = ConnectionProperties::default();
        let connection = Connection::connect(&amqp_addr, conn_options)
            .await
            .expect("Failed to connect to testcontainer RabbitMQ");

        // Create channel and declare exchange here
        let publish_channel = connection
            .create_channel()
            .await
            .expect("Failed to create publish channel in setup");

        publish_channel
            .exchange_declare(
                exchange_name,
                ExchangeKind::Topic,
                ExchangeDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .expect("Failed to declare exchange in setup");

        let bus = RabbitMqEventBus {
            connection: Arc::new(connection),
            publish_channel: Arc::new(publish_channel),
            exchange_name: exchange_name.to_string(),
        };

        (bus, node)
    }

    // Helper to setup consumer for tests - returns a Stream
    async fn setup_consumer(
        channel: &Channel,
        exchange: &str,
        queue_name: &str,
        routing_key: &str,
        // Return the stream directly
    ) -> Result<impl Stream<Item = Result<Delivery, lapin::Error>>, Box<dyn std::error::Error>>
    {
        channel
            .queue_declare(
                queue_name,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;
        channel
            .queue_bind(
                queue_name,
                exchange,
                routing_key,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;
        let consumer = channel
            .basic_consume(
                queue_name,
                "test_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        Ok(consumer) // basic_consume returns the Consumer which implements Stream
    }

    #[tokio::test]
    async fn test_publish_and_consume_rabbitmq() {
        let exchange_name = "test_exchange_bus";
        // Get the bus directly from the modified setup function
        let (bus, _node) = setup_rabbitmq_bus(exchange_name).await;

        let routing_key = "test.event.routing";
        let event_type = "TestEventOccurred";
        let payload = b"rabbitmq test payload".to_vec();

        let consumer_channel = bus
            .create_subscriber_channel()
            .await
            .expect("Failed to create consumer channel");
        let queue_name = "test_queue_bus";
        // Get the stream directly
        let mut consumer_stream =
            setup_consumer(&consumer_channel, exchange_name, queue_name, routing_key)
                .await
                .expect("Failed to setup consumer");

        // Publish the event
        let publish_result = bus.publish(routing_key, event_type, &payload).await;
        assert!(
            publish_result.is_ok(),
            "Publish failed: {:?}",
            publish_result.err()
        );

        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(1);
        tokio::spawn(async move {
            // Use the stream here
            if let Some(delivery_result) = consumer_stream.next().await {
                match delivery_result {
                    Ok(delivery) => {
                        delivery.ack(Default::default()).await.expect("Ack failed");
                        tx.send(delivery.data).await.expect("Send to test failed");
                    }
                    Err(e) => eprintln!("Consume error: {:?}", e),
                }
            }
        });

        match tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv()).await {
            Ok(Some(received_payload)) => assert_eq!(received_payload, payload),
            Ok(None) => panic!("Consumer channel closed unexpectedly"),
            Err(_) => panic!("Test timed out waiting for message"),
        }
    }
}
