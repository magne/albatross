use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// Embed migrations
mod migrations {
    refinery::embed_migrations!("./migrations");
}

use core_lib::CoreError;
use core_lib::adapters::in_memory_event_bus::{InMemoryEventBus, InMemoryMessage};
use prost::Message;
use proto::{
    tenant::TenantCreated,
    user::{Role, UserRegistered},
};
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing (logging)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()) // Allow RUST_LOG env var
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Starting Projection Worker...");

    // TODO: Load configuration (DB connection string, event bus details, etc.)

    // TODO: Set up database connection pool

    // TODO: Run database migrations using Refinery
    // Example (requires db client setup):
    // let mut client = ...; // Get a tokio_postgres::Client
    // migrations::migrations::runner().run_async(&mut client).await?;
    // info!("Database migrations applied successfully.");

    // Set up event bus subscriber (using InMemoryEventBus for now)
    let event_bus = Arc::new(InMemoryEventBus::default());
    // Subscribe to relevant topics
    // TODO: Get topic names from config or constants
    let mut tenant_rx = event_bus.subscribe("tenant_events");
    let mut user_rx = event_bus.subscribe("user_events");

    // TODO: Set up Redis connection/client for publishing notifications
    // let redis_publisher: Arc<dyn EventPublisher> = Arc::new(RedisPublisher::new(...));

    info!("Projection Worker started. Listening for events...");

    // Start the main event consumption loop
    loop {
        tokio::select! {
            // Biased select ensures we check topics somewhat fairly,
            // but consider dedicated tasks per topic/subscription in production.
            biased;

            result = tenant_rx.recv() => {
                match result {
                    Ok(message) => {
                        info!("Received tenant event: Type = {}, Topic = {}", message.event_type, message.topic);
                        handle_event(message).await?; // Pass message to generic handler
                    },
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!("Tenant event receiver lagged by {} messages", n);
                    },
                    Err(RecvError::Closed) => {
                        tracing::error!("Tenant event channel closed.");
                        break; // Exit loop if channel closes
                    }
                }
            },
            result = user_rx.recv() => {
                 match result {
                    Ok(message) => {
                        info!("Received user event: Type = {}, Topic = {}", message.event_type, message.topic);
                        handle_event(message).await?; // Pass message to generic handler
                    },
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!("User event receiver lagged by {} messages", n);
                    },
                    Err(RecvError::Closed) => {
                        tracing::error!("User event channel closed.");
                        break; // Exit loop if channel closes
                    }
                }
            },
        }
    }

    // info!("Projection Worker finished."); // Loop runs indefinitely unless channel closes

    Ok(())
}

// --- Event Handling Logic ---

async fn handle_event(message: InMemoryMessage) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Inject DB pool and Redis publisher here
    // let db_pool = ...;
    // let redis_publisher = ...;

    match message.event_type.as_str() {
        "TenantCreated" => match TenantCreated::decode(message.payload.as_slice()) {
            Ok(event) => handle_tenant_created(event).await?,
            Err(e) => tracing::error!("Failed to decode TenantCreated: {}", e),
        },
        "UserRegistered" => match UserRegistered::decode(message.payload.as_slice()) {
            Ok(event) => handle_user_registered(event).await?,
            Err(e) => tracing::error!("Failed to decode UserRegistered: {}", e),
        },
        // Add other event types later (PasswordChanged, ApiKeyGenerated, PIREPSubmitted, etc.)
        _ => {
            tracing::warn!("Received unknown event type: {}", message.event_type);
        }
    }
    Ok(())
}

// --- Projection Handlers ---

async fn handle_tenant_created(event: TenantCreated) -> Result<(), CoreError> {
    info!(
        "Projecting TenantCreated: ID = {}, Name = {}",
        event.tenant_id, event.name
    );
    // TODO: Implement database logic to INSERT into tenants table
    // let query = "INSERT INTO tenants (tenant_id, name) VALUES ($1, $2)";
    // db_pool.execute(query, &[&event.tenant_id, &event.name]).await?;

    // TODO: Publish notification to Redis Pub/Sub
    // let notification_topic = format!("tenant:{}:updates", event.tenant_id);
    // let notification_payload = serde_json::to_vec(&event)?; // Or a specific notification format
    // redis_publisher.publish(&notification_topic, "TenantCreated", &notification_payload).await?;

    Ok(())
}

async fn handle_user_registered(event: UserRegistered) -> Result<(), CoreError> {
    info!(
        "Projecting UserRegistered: ID = {}, Username = {}, Role = {:?}, Tenant = {:?}",
        event.user_id,
        event.username,
        Role::try_from(event.role),
        event.tenant_id
    );
    // TODO: Implement database logic to INSERT into users table
    // Need to handle password hash - it's NOT in the event. Query event store? Or assume command handler stored it?
    // For now, assume hash needs separate handling or is added later.
    // let role_str = format!("{:?}", Role::try_from(event.role).unwrap_or(Role::Unspecified));
    // let query = "INSERT INTO users (user_id, tenant_id, username, email, role, password_hash) VALUES ($1, $2, $3, $4, $5, $6)";
    // db_pool.execute(query, &[&event.user_id, &event.tenant_id, &event.username, &event.email, &role_str, &"PLACEHOLDER_HASH"]).await?;

    // TODO: Publish notification to Redis Pub/Sub
    // let notification_topic = format!("user:{}:updates", event.user_id);
    // let notification_payload = serde_json::to_vec(&event)?;
    // redis_publisher.publish(&notification_topic, "UserRegistered", &notification_payload).await?;

    Ok(())
}
