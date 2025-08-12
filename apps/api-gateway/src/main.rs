// Import necessary items from the crate's library (lib.rs)
use api_gateway::{AppState, create_app};
use core_lib::{
    Cache, EventPublisher, Repository,
    adapters::{
        in_memory_cache::InMemoryCache, in_memory_event_bus::InMemoryEventBus,
        in_memory_repository::InMemoryEventRepository,
    },
};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tracing::{Level, info, warn};
use sqlx::postgres::PgPoolOptions;
use dotenvy::dotenv;
use tracing_subscriber::FmtSubscriber;

// main.rs now only contains the binary entry point and setup specific to running the application.
// All shared application logic (router creation, state, handlers) is in lib.rs.

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
    // TODO: Replace with configuration loading and real adapters (Postgres, RabbitMQ, Redis)
    // For now, using in-memory adapters like in tests.
    let user_repo: Arc<dyn Repository> = Arc::new(InMemoryEventRepository::default());
    let tenant_repo: Arc<dyn Repository> = Arc::new(InMemoryEventRepository::default());
    let event_bus: Arc<dyn EventPublisher> = Arc::new(InMemoryEventBus::default());
    let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::default());

    // Create the application state using the struct from lib.rs
    let app_state = AppState {
        user_repo: user_repo.clone(),
        tenant_repo: tenant_repo.clone(),
        event_bus: event_bus.clone(),
        cache: cache.clone(),
        pg_pool,
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
