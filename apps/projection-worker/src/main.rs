use core_lib::adapters::rabbitmq_event_bus::RabbitMqEventBus;
use core_lib::adapters::redis_event_bus::RedisEventBus;
use core_lib::{CoreError, EventPublisher}; // Remove EventSubscriber import
use dotenvy::dotenv;
use futures_util::StreamExt; // Required for consumer.next()
use lapin::{
    Channel,
    Consumer, // Import lapin types
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
};
use prost::Message;
use proto::{
    tenant::TenantCreated,
    user::{Role, UserRegistered},
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::sync::Arc;
// use tokio::sync::broadcast::error::RecvError; // Not needed for lapin consumer stream
// use tokio_postgres::NoTls; // No longer needed directly here
use tracing::{Level, error, info, warn};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid; // Needed for parsing UUIDs in handlers

// Embed migrations
mod migrations {
    refinery::embed_migrations!("./migrations");
}

// Define a generic error type for the main function
type BoxError = Box<dyn std::error::Error + Send + Sync>;

// --- Migration Runner ---
// Runs migrations using a separate tokio_postgres client
// Returns BoxError for easier handling in main
async fn run_migrations(db_url: &str) -> Result<(), BoxError> {
    // Create a connection config
    let config: tokio_postgres::Config = db_url.parse()?; // Let ? handle BoxError conversion

    // Create the client and connection
    let (mut client, connection) = config
        .connect(tokio_postgres::NoTls) // Assuming NoTls for local dev; configure TLS appropriately for production
        .await?; // Let ? handle BoxError conversion

    // Spawn the connection task
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("PostgreSQL connection error (migrations): {}", e);
        }
    });

    // Run the migrations
    info!("Applying database migrations...");
    let report = migrations::migrations::runner()
        .run_async(&mut client)
        .await?; // Let ? handle BoxError conversion

    if !report.applied_migrations().is_empty() {
        info!(
            "Applied {} migrations: {:?}",
            report.applied_migrations().len(),
            report
                .applied_migrations()
                .iter()
                .map(|m| m.name())
                .collect::<Vec<_>>()
        );
    } else {
        info!("No new migrations to apply.");
    }

    // Close the client (optional, happens on drop)
    // client.close().await?;

    Ok(()) // Return Ok(()) on success
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize tracing (logging)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO) // TODO: Make log level configurable
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()) // Allow RUST_LOG
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!(
        "Starting Projection Worker v{}...",
        env!("CARGO_PKG_VERSION")
    );

    // --- Configuration ---
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let rabbitmq_url = env::var("RABBITMQ_URL").expect("RABBITMQ_URL must be set");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    let exchange_name =
        env::var("RABBITMQ_EXCHANGE_NAME").unwrap_or_else(|_| "albatross_exchange".to_string());
    // Queue names can be derived or configured
    let tenant_queue = "projection_worker_tenant_queue";
    let user_queue = "projection_worker_user_queue";
    // Routing keys should match what the publisher uses (e.g., "tenant.*", "user.*")
    let tenant_routing_key = "tenant.*"; // Listen for all tenant events
    let user_routing_key = "user.*"; // Listen for all user events

    // --- Database Setup & Migrations ---
    // Run migrations using a separate client connection
    if let Err(e) = run_migrations(&database_url).await {
        error!("Database migration failed: {}", e);
        return Err(e); // Exit if migrations fail
    }

    // Create the main database connection pool for the application
    let db_pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(10) // TODO: Make pool size configurable
            .connect(&database_url)
            .await?,
    );
    info!("Database connection pool established.");

    // --- Event Bus Setup ---
    // Subscriber Bus (used for creating channels)
    let subscriber_bus = Arc::new(
        RabbitMqEventBus::new(&rabbitmq_url, &exchange_name) // Pass exchange name
            .await
            .expect("Failed to connect to RabbitMQ"),
    );
    info!("Connected to RabbitMQ.");

    // Publisher (Redis for Notifications)
    let notification_publisher: Arc<dyn EventPublisher> = Arc::new(
        RedisEventBus::new(&redis_url, None) // Provide None for optional channel_prefix
            .await
            .expect("Failed to connect to Redis"),
    );
    info!("Connected to Redis for publishing notifications.");

    // --- Setup Consumers ---
    let tenant_consumer: Consumer = setup_consumer(
        // Add explicit type annotation
        subscriber_bus.create_subscriber_channel().await?,
        &exchange_name,
        tenant_queue,
        tenant_routing_key, // Use routing key for binding
        "tenant_consumer_tag",
    )
    .await?;
    info!("Tenant event consumer ready.");

    let user_consumer: Consumer = setup_consumer(
        // Add explicit type annotation
        subscriber_bus.create_subscriber_channel().await?,
        &exchange_name,
        user_queue,
        user_routing_key, // Use routing key for binding
        "user_consumer_tag",
    )
    .await?;
    info!("User event consumer ready.");

    info!("Projection Worker started successfully. Listening for events...");

    // --- Main Event Consumption Loop ---
    let mut tenant_consumer = tenant_consumer; // Make mutable for next()
    let mut user_consumer = user_consumer; // Make mutable for next()

    loop {
        tokio::select! {
            biased; // Ensure fairness between queues

            // --- Tenant Events ---
            maybe_delivery = tenant_consumer.next() => {
                if let Some(delivery_result) = maybe_delivery {
                    match delivery_result {
                        Ok(delivery) => {
                            info!("Received tenant event delivery.");
                            // Attempt to get event type from headers again
                            let event_type = delivery
                                .properties
                                .headers() // Returns Option<&FieldTable>
                                .as_ref()
                                .and_then(|headers| headers.inner().get("event_type")) // Look for "event_type" key
                                .and_then(|header_value| match header_value {
                                    lapin::types::AMQPValue::LongString(s) => Some(s.to_string()), // Expecting a string
                                    _ => None,
                                })
                                .unwrap_or_else(|| {
                                    warn!("Received message without 'event_type' header");
                                    String::new()
                                });
                            let payload = delivery.data.clone(); // Clone payload for handler

                            if event_type.is_empty() {
                                error!("Cannot handle message without event type. NACKing.");
                                if let Err(nack_err) = delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await {
                                     error!("Failed to NACK message without type: {}", nack_err);
                                }
                                continue; // Skip to next iteration
                            }

                            match handle_event(payload, event_type.clone(), Arc::clone(&db_pool), Arc::clone(&notification_publisher)).await {
                                Ok(_) => {
                                    if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                                        error!("Failed to ACK tenant event: {}", e);
                                    } else {
                                         info!("Tenant event processed and ACKed.");
                                    }
                                }
                                Err(e) => {
                                    error!("Error handling tenant event: {}. NACKing...", e);
                                    // Nack without requeueing to avoid poison messages
                                    if let Err(nack_err) = delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await {
                                        error!("Failed to NACK tenant event: {}", nack_err);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error receiving tenant delivery: {}", e);
                            // Consider reconnection logic here
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        }
                    }
                } else {
                     warn!("Tenant consumer stream ended.");
                     break; // Or implement reconnection logic
                }
            },

             // --- User Events --- (Similar logic as Tenant Events)
            maybe_delivery = user_consumer.next() => {
                if let Some(delivery_result) = maybe_delivery {
                    match delivery_result {
                        Ok(delivery) => {
                            info!("Received user event delivery.");
                             // Attempt to get event type from headers again
                            let event_type = delivery
                                .properties
                                .headers() // Returns Option<&FieldTable>
                                .as_ref()
                                .and_then(|headers| headers.inner().get("event_type")) // Look for "event_type" key
                                .and_then(|header_value| match header_value {
                                    lapin::types::AMQPValue::LongString(s) => Some(s.to_string()), // Expecting a string
                                    _ => None,
                                })
                                .unwrap_or_else(|| {
                                    warn!("Received message without 'event_type' header");
                                    String::new()
                                });
                            let payload = delivery.data.clone();

                             if event_type.is_empty() {
                                error!("Cannot handle message without event type. NACKing.");
                                if let Err(nack_err) = delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await {
                                     error!("Failed to NACK message without type: {}", nack_err);
                                }
                                continue; // Skip to next iteration
                            }

                            match handle_event(payload, event_type.clone(), Arc::clone(&db_pool), Arc::clone(&notification_publisher)).await {
                                Ok(_) => {
                                    if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                                        error!("Failed to ACK user event: {}", e);
                                    } else {
                                         info!("User event processed and ACKed.");
                                    }
                                }
                                Err(e) => {
                                    error!("Error handling user event: {}. NACKing...", e);
                                    if let Err(nack_err) = delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await {
                                        error!("Failed to NACK user event: {}", nack_err);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error receiving user delivery: {}", e);
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        }
                    }
                } else {
                     warn!("User consumer stream ended.");
                     break; // Or implement reconnection logic
                }
            },
        }
    }

    info!("Projection Worker event loop finished."); // Should only happen on break/error

    Ok(())
}

// --- Consumer Setup Helper ---
async fn setup_consumer(
    channel: Channel,
    exchange: &str,
    queue_name: &str,
    routing_key: &str,
    consumer_tag: &str,
) -> Result<Consumer, lapin::Error> {
    // Declare the queue
    channel
        .queue_declare(
            queue_name,
            QueueDeclareOptions {
                durable: true, // Ensure queue survives restarts
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;
    info!("Queue '{}' declared.", queue_name);

    // Bind the queue to the exchange
    channel
        .queue_bind(
            queue_name,
            exchange,
            routing_key, // Use the specific routing key for binding
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;
    info!(
        "Queue '{}' bound to exchange '{}' with key '{}'.",
        queue_name, exchange, routing_key
    );

    // Start consuming messages
    let consumer = channel
        .basic_consume(
            queue_name,
            consumer_tag, // Unique consumer tag
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;
    Ok(consumer)
}

// --- Event Handling Logic ---
async fn handle_event(
    payload: Vec<u8>,
    event_type: String,
    db_pool: Arc<PgPool>,
    publisher: Arc<dyn EventPublisher>,
) -> Result<(), BoxError> {
    // Return BoxError for easier ? usage
    info!("Handling event type: {}", event_type);
    match event_type.as_str() {
        // TODO: Use constants for event type strings
        "TenantCreated" => match TenantCreated::decode(payload.as_slice()) {
            Ok(event) => {
                handle_tenant_created(event, Arc::clone(&db_pool), Arc::clone(&publisher)) // Pass Arc clones
                    .await
                    .map_err(BoxError::from)?
            } // Convert CoreError to BoxError
            Err(e) => error!("Failed to decode TenantCreated: {}", e), // Keep logging, but don't return error here
        },
        "UserRegistered" => match UserRegistered::decode(payload.as_slice()) {
            Ok(event) => {
                handle_user_registered(event, Arc::clone(&db_pool), Arc::clone(&publisher)) // Pass Arc clones
                    .await
                    .map_err(BoxError::from)?
            } // Convert CoreError to BoxError
            Err(e) => error!("Failed to decode UserRegistered: {}", e),
        },
        // TODO: Add other event types (PasswordChanged, ApiKeyGenerated, PIREPSubmitted, etc.)
        _ => {
            warn!("Received unknown event type: {}", event_type); // Use correct variable name
        }
    }
    Ok(())
}

// --- Projection Handlers ---

async fn handle_tenant_created(
    event: TenantCreated,
    db_pool: Arc<PgPool>,
    publisher: Arc<dyn EventPublisher>,
) -> Result<(), CoreError> {
    info!(
        "Projecting TenantCreated: ID = {}, Name = {}",
        event.tenant_id, event.name
    );
    // TODO: Implement database logic to INSERT into tenants table using sqlx
    let tenant_uuid = Uuid::parse_str(&event.tenant_id).map_err(|e| {
        CoreError::Deserialization(format!(
            "Invalid tenant_id UUID: {} - {}",
            event.tenant_id, e
        ))
    })?;

    sqlx::query!(
        "INSERT INTO tenants (tenant_id, name, created_at, updated_at) VALUES ($1::Uuid, $2, NOW(), NOW())",
        tenant_uuid,
        event.name
    )
    .execute(db_pool.as_ref()) // Use as_ref() to get &PgPool
    .await
    .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;

    info!("Tenant {} inserted into read model.", event.tenant_id);

    // Publish notification to Redis Pub/Sub
    let notification_topic = format!("tenant:{}:updates", event.tenant_id);
    // Using JSON for notification payload for now
    let notification_payload = serde_json::to_vec(&event).map_err(|e| {
        CoreError::Serialization(format!("Failed to serialize TenantCreated: {}", e))
    })?;
    publisher
        .publish(&notification_topic, "TenantCreated", &notification_payload)
        .await?;
    info!(
        "Published TenantCreated notification to topic: {}",
        notification_topic
    );

    Ok(())
}

async fn handle_user_registered(
    event: UserRegistered,
    db_pool: Arc<PgPool>,
    publisher: Arc<dyn EventPublisher>,
) -> Result<(), CoreError> {
    info!(
        "Projecting UserRegistered: ID = {}, Username = {}, Role = {:?}, Tenant = {:?}",
        event.user_id,
        event.username,
        Role::try_from(event.role),
        event.tenant_id
    );

    let user_uuid = Uuid::parse_str(&event.user_id).map_err(|e| {
        CoreError::Deserialization(format!("Invalid user_id UUID: {} - {}", event.user_id, e))
    })?;
    let tenant_uuid = event
        .tenant_id
        .as_ref()
        .map(|id| Uuid::parse_str(id))
        .transpose() // Convert Option<Result<Uuid, _>> to Result<Option<Uuid>, _>
        .map_err(|e| {
            CoreError::Deserialization(format!(
                "Invalid tenant_id UUID: {:?} - {}",
                event.tenant_id, e
            ))
        })?;
    let role_enum = Role::try_from(event.role).unwrap_or(Role::Unspecified);
    let role_str = role_enum.as_str_name(); // Get Protobuf enum string name

    // Note: Password hash is NOT in the event. Inserting placeholder.
    // This read model might need adjustment later depending on how auth is handled.
    sqlx::query!(
        "INSERT INTO users (user_id, tenant_id, username, email, role, password_hash, created_at, updated_at)
         VALUES ($1::Uuid, $2::Uuid, $3, $4, $5, $6, NOW(), NOW())", // Add ::Uuid hints
        user_uuid,
        tenant_uuid, // sqlx handles Option<Uuid> correctly, hint should work
        event.username,
        event.email,
        role_str,
        "PLACEHOLDER_HASH" // Password hash is not available in the event
    )
    .execute(db_pool.as_ref()) // Use as_ref() to get &PgPool
    .await
    .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;

    info!("User {} inserted into read model.", event.user_id);

    // Publish notification to Redis Pub/Sub
    let notification_topic = format!("user:{}:updates", event.user_id);
    // Using JSON for notification payload for now
    let notification_payload = serde_json::to_vec(&event).map_err(|e| {
        CoreError::Serialization(format!("Failed to serialize UserRegistered: {}", e))
    })?;
    publisher
        .publish(&notification_topic, "UserRegistered", &notification_payload)
        .await?;
    info!(
        "Published UserRegistered notification to topic: {}",
        notification_topic
    );

    Ok(())
}
