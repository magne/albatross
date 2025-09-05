// Import necessary items from the crate's library (lib.rs)
use api_gateway::{AppState, create_app};
use core_lib::{
    Cache, EventPublisher, Repository,
    adapters::{
        in_memory_cache::InMemoryCache,
        postgres_repository::PostgresEventRepository,
        rabbitmq_event_bus::RabbitMqEventBus,
    },
};
use std::{env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tracing::{Level, info, warn, error};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use dotenvy::dotenv;
use tracing_subscriber::FmtSubscriber;

// main.rs now only contains the binary entry point and setup specific to running the application.
// All shared application logic (router creation, state, handlers) is in lib.rs.

// --- Migration Runner ---
// Runs migrations using sqlx migrate
async fn run_migrations(db_url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a connection pool for migrations
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(db_url)
        .await?;

    // Run the migrations
    info!("Applying database migrations...");
    let migrator = sqlx::migrate::Migrator::new(std::path::Path::new("./migrations")).await?;
    migrator.run(&pool).await?;

    info!("Migrations applied successfully.");

    // Verify tables exist after migrations
    let tables = sqlx::query("SELECT tablename FROM pg_tables WHERE schemaname = 'public' AND tablename IN ('events', 'tenants', 'users', 'user_api_keys')")
        .fetch_all(&pool)
        .await?;

    let existing_tables: Vec<String> = tables
        .iter()
        .map(|row| row.get::<String, &str>("tablename"))
        .collect();

    if existing_tables.len() < 4 {
        warn!("Migration completed but some tables are missing. Expected: events, tenants, users, user_api_keys. Found: {:?}", existing_tables);
        // Don't fail here - let the application handle missing tables gracefully
    } else {
        info!("Migration verification successful - all required tables exist: {:?}", existing_tables);
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // Initialize tracing (logging)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO) // TODO: Make log level configurable
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()) // Allow RUST_LOG
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Starting API Gateway v{}...", env!("CARGO_PKG_VERSION"));
    // Load environment (.env) if present
    dotenv().ok();

    // Optional Postgres pool for query endpoints (Step 9)
    let pg_pool = match std::env::var("DATABASE_URL") {
        Ok(url) => {
            match PgPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await {
                    Ok(pool) => {
                        info!("Connected to Postgres for read model queries");

                        // Run migrations if database is available
                        if let Err(e) = run_migrations(&url).await {
                            error!("Database migration failed: {}", e);
                            return; // Exit if migrations fail - database is in inconsistent state
                        }

                        Some(pool)
                    }
                    Err(e) => {
                        warn!("Failed to connect to Postgres (queries disabled): {}", e);
                        None
                    }
                }
        }
        Err(_) => {
            warn!("DATABASE_URL not set (queries disabled)");
            None
        }
    };

    // --- Dependency Injection Setup ---
    // Using PostgreSQL repositories for persistence
    let db_pool = match pg_pool {
        Some(pool) => pool,
        None => {
            error!("DATABASE_URL not set - PostgreSQL required for event persistence");
            return;
        }
    };

    let user_repo: Arc<dyn Repository> = Arc::new(PostgresEventRepository::new(db_pool.clone()));
    let tenant_repo: Arc<dyn Repository> = Arc::new(PostgresEventRepository::new(db_pool.clone()));

    // Configure RabbitMQ event bus (same as projection-worker)
    let rabbitmq_url = env::var("RABBITMQ_URL").unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());
    let exchange_name = env::var("RABBITMQ_EXCHANGE_NAME").unwrap_or_else(|_| "albatross_exchange".to_string());

    let event_bus: Arc<dyn EventPublisher> = Arc::new(
        RabbitMqEventBus::new(&rabbitmq_url, &exchange_name)
            .await
            .expect("Failed to connect to RabbitMQ")
    );
    info!("Connected to RabbitMQ for event publishing");

    let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::default());

    // Create the application state using the struct from lib.rs
    // Optional Redis client for WebSocket real-time (Step 10)
    let redis_client = match std::env::var("REDIS_URL") {
        Ok(url) => {
            match redis::Client::open(url.clone()) {
                Ok(c) => {
                    info!("Connected to Redis for WebSocket real-time");
                    Some(c)
                }
                Err(e) => {
                    warn!("Failed to create Redis client (WS real-time disabled): {}", e);
                    None
                }
            }
        }
        Err(_) => {
            warn!("REDIS_URL not set (WS real-time disabled)");
            None
        }
    };

    let app_state = AppState {
        user_repo: user_repo.clone(),
        tenant_repo: tenant_repo.clone(),
        event_bus: event_bus.clone(),
        cache: cache.clone(),
        pg_pool: Some(db_pool),
        redis_client,
    };

    // Create the application router using the function from lib.rs
    let app = create_app(app_state);

    // Define the address and port to run the server on
    // TODO: Make address/port configurable
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("API Gateway listening on {}", addr);

    // Create a TCP listener
    let listener = TcpListener::bind(addr).await.unwrap_or_else(|e| {
        panic!("Failed to bind to address {}: {}", addr, e);
    });

    // Run the server
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap_or_else(|e| {
            panic!("Server failed to run: {}", e);
        });
}
