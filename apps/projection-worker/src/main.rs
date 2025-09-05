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
    user::{ApiKeyGenerated, ApiKeyRevoked, Role, UserRegistered}, // Added ApiKey events
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::env;
use std::sync::Arc;
// use tokio::sync::broadcast::error::RecvError; // Not needed for lapin consumer stream
// use tokio_postgres::NoTls; // No longer needed directly here
use tracing::{Level, error, info, warn};
use chrono::Utc;
use serde_json::json;
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid; // Needed for parsing UUIDs in handlers

// Migrations are handled by sqlx migrate

// Define a generic error type for the main function
type BoxError = Box<dyn std::error::Error + Send + Sync>;

// Migrations are now handled by api-gateway (command side)

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

    // --- Database Setup ---
    // Migrations are now handled by api-gateway. Verify tables exist.
    let temp_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;

    // Verify required tables exist (created by api-gateway migrations)
    let tables = sqlx::query("SELECT tablename FROM pg_tables WHERE schemaname = 'public' AND tablename IN ('events', 'tenants', 'users', 'user_api_keys')")
        .fetch_all(&temp_pool)
        .await?;

    let existing_tables: Vec<String> = tables
        .iter()
        .map(|row| row.get::<String, &str>("tablename"))
        .collect();

    if existing_tables.len() < 4 {
        error!("Required tables missing. Expected: events, tenants, users, user_api_keys. Found: {:?}. Ensure api-gateway has started and created the database schema.", existing_tables);
        return Err("Database schema not ready - start api-gateway first".into());
    } else {
        info!("Database schema verification successful - all required tables exist: {:?}", existing_tables);
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
                                .kind().clone() // Get message type from properties
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| {
                                    warn!("Received message without message type");
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
                                .kind().clone() // Get message type from properties
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| {
                                    warn!("Received message without message type");
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
        "ApiKeyGenerated" => match ApiKeyGenerated::decode(payload.as_slice()) {
            Ok(event) => {
                handle_api_key_generated(event, Arc::clone(&db_pool), Arc::clone(&publisher))
                    .await
                    .map_err(BoxError::from)?
            }
            Err(e) => error!("Failed to decode ApiKeyGenerated: {}", e),
        },
        "ApiKeyRevoked" => match ApiKeyRevoked::decode(payload.as_slice()) {
            Ok(event) => {
                handle_api_key_revoked(event, Arc::clone(&db_pool), Arc::clone(&publisher))
                    .await
                    .map_err(BoxError::from)?
            }
            Err(e) => error!("Failed to decode ApiKeyRevoked: {}", e),
        },
        // TODO: Add other event types (PasswordChanged, PIREPSubmitted, etc.)
        _ => {
            warn!("Received unknown event type: {}", event_type);
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
    let envelope = json!({
        "event_type": "TenantCreated",
        "ts": Utc::now().to_rfc3339(),
        "data": &event,
        "meta": {
            "tenant_id": event.tenant_id,
            "aggregate_id": event.tenant_id,
            "version": serde_json::Value::Null
        }
    });
    let notification_payload = serde_json::to_vec(&envelope).map_err(|e| {
        CoreError::Serialization(format!("Failed to serialize TenantCreated envelope: {}", e))
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

    sqlx::query!(
        "INSERT INTO users (user_id, tenant_id, username, email, role, password_hash, created_at, updated_at)
         VALUES ($1::Uuid, $2::Uuid, $3, $4, $5, $6, NOW(), NOW())", // Add ::Uuid hints
        user_uuid,
        tenant_uuid, // sqlx handles Option<Uuid> correctly, hint should work
        event.username,
        event.email,
        role_str,
        event.password_hash // Use password hash from the event
    )
    .execute(db_pool.as_ref()) // Use as_ref() to get &PgPool
    .await
    .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;

    info!("User {} inserted into read model.", event.user_id);

    // Publish notification to Redis Pub/Sub
    let notification_topic = format!("user:{}:updates", event.user_id);
    // Using JSON for notification payload for now
    let envelope = json!({
        "event_type": "UserRegistered",
        "ts": Utc::now().to_rfc3339(),
        "data": &event,
        "meta": {
            "tenant_id": event.tenant_id.clone(),
            "aggregate_id": event.user_id,
            "version": serde_json::Value::Null
        }
    });
    let notification_payload = serde_json::to_vec(&envelope).map_err(|e| {
        CoreError::Serialization(format!("Failed to serialize UserRegistered envelope: {}", e))
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

async fn handle_api_key_generated(
    event: ApiKeyGenerated,
    db_pool: Arc<PgPool>,
    publisher: Arc<dyn EventPublisher>,
) -> Result<(), CoreError> {
    info!(
        "Projecting ApiKeyGenerated: UserID = {}, KeyID = {}, Name = {}",
        event.user_id, event.key_id, event.key_name
    );

    let user_uuid = Uuid::parse_str(&event.user_id).map_err(|e| {
        CoreError::Deserialization(format!("Invalid user_id UUID: {} - {}", event.user_id, e))
    })?;
    // Assuming tenant_id might be associated with the user, fetch it if needed or pass if available in event
    // For now, let's assume we might need to look it up or it's nullable in the keys table.
    // Fetching tenant_id from users table for denormalization:
    // Use event.user_id (String) in the WHERE clause, cast placeholder
    let tenant_id_row = sqlx::query!(
        "SELECT tenant_id::TEXT FROM users WHERE user_id = $1::VARCHAR", // Select as TEXT, Cast $1
        event.user_id // Pass the original String ID
    )
    .fetch_optional(db_pool.as_ref())
    .await
    .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;

    // Fetch as Option<String>, then parse to Option<Uuid>
    let tenant_uuid: Option<Uuid> = tenant_id_row
        .and_then(|row| row.tenant_id) // Get Option<String>
        .map(|id_str| Uuid::parse_str(id_str.as_str())) // Map String to Result<Uuid, _>
        .transpose() // Convert Option<Result<Uuid, _>> to Result<Option<Uuid>, _>
        .map_err(|e| {
            CoreError::Deserialization(format!("Invalid tenant_id UUID fetched from DB: {}", e))
        })?;

    sqlx::query!(
        "INSERT INTO user_api_keys (key_id, user_id, tenant_id, key_name, api_key_hash, created_at)
         VALUES ($1, $2::Uuid, $3::Uuid, $4, $5, NOW())",
        event.key_id,
        user_uuid,
        tenant_uuid, // Insert fetched tenant_id
        event.key_name,
        event.api_key_hash // Use the hash from the event
    )
    .execute(db_pool.as_ref())
    .await
    .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;

    info!(
        "API Key {} for user {} inserted into read model.",
        event.key_id, event.user_id
    );

    // Publish notification
    let notification_topic = format!("user:{}:apikeys", event.user_id);
    let envelope = json!({
        "event_type": "ApiKeyGenerated",
        "ts": Utc::now().to_rfc3339(),
        "data": &event,
        "meta": {
            "tenant_id": tenant_uuid.map(|u| u.to_string()),
            "aggregate_id": event.user_id,
            "version": serde_json::Value::Null
        }
    });
    let notification_payload = serde_json::to_vec(&envelope).map_err(|e| {
        CoreError::Serialization(format!("Failed to serialize ApiKeyGenerated envelope: {}", e))
    })?;
    publisher
        .publish(
            &notification_topic,
            "ApiKeyGenerated",
            &notification_payload,
        )
        .await?;
    info!(
        "Published ApiKeyGenerated notification to topic: {}",
        notification_topic
    );

    Ok(())
}

async fn handle_api_key_revoked(
    event: ApiKeyRevoked,
    db_pool: Arc<PgPool>,
    publisher: Arc<dyn EventPublisher>,
) -> Result<(), CoreError> {
    info!(
        "Projecting ApiKeyRevoked: UserID = {}, KeyID = {}",
        event.user_id, event.key_id
    );

    // No need to parse user_id as UUID here unless we need it for validation/logging
    // key_id is the primary key for the update

    let result = sqlx::query!(
        // Compare user_id (VARCHAR) with the String parameter $2
        "UPDATE user_api_keys SET revoked_at = NOW() WHERE key_id = $1 AND user_id = $2",
        event.key_id,
        event.user_id // Pass the user_id String directly
    )
    .execute(db_pool.as_ref())
    .await
    .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;

    if result.rows_affected() == 0 {
        warn!(
            "API Key {} not found or already revoked for user {}.",
            event.key_id, event.user_id
        );
        // Decide if this should be an error or just a warning
        // return Err(CoreError::NotFound(format!("API Key {} not found for user {}", event.key_id, event.user_id)));
    } else {
        info!(
            "API Key {} for user {} marked as revoked in read model.",
            event.key_id, event.user_id
        );
    }

    // Publish notification
    let notification_topic = format!("user:{}:apikeys", event.user_id);
    let envelope = json!({
        "event_type": "ApiKeyRevoked",
        "ts": Utc::now().to_rfc3339(),
        "data": &event,
        "meta": {
            "tenant_id": serde_json::Value::Null, // Not readily available; could be enriched later
            "aggregate_id": event.user_id,
            "version": serde_json::Value::Null
        }
    });
    let notification_payload = serde_json::to_vec(&envelope).map_err(|e| {
        CoreError::Serialization(format!("Failed to serialize ApiKeyRevoked envelope: {}", e))
    })?;
    publisher
        .publish(&notification_topic, "ApiKeyRevoked", &notification_payload)
        .await?;
    info!(
        "Published ApiKeyRevoked notification to topic: {}",
        notification_topic
    );

    Ok(())
}
