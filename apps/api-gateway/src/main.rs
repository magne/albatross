use axum::{
    Router,
    body::Body,
    http::{HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::{get, post}, // Added post
};
use core_lib::{
    Cache,
    // Import core traits and in-memory adapters
    EventPublisher,
    Repository,
    adapters::{
        in_memory_cache::InMemoryCache, in_memory_event_bus::InMemoryEventBus,
        in_memory_repository::InMemoryEventRepository,
    },
    domain::{tenant::Tenant, user::User}, // Import aggregates
};
use rust_embed::RustEmbed;
use std::{net::SocketAddr, sync::Arc}; // Added Arc
use tokio::net::TcpListener;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// Declare application modules
mod application;
use application::commands::{
    create_tenant::handle_create_tenant_request,
    // Import handler functions
    register_user::handle_register_user_request,
};

// Define the struct to embed frontend assets
// Assumes the frontend build output is in `../web-ui/dist` relative to this crate's Cargo.toml
#[derive(RustEmbed)]
#[folder = "../web-ui/dist/"]
#[include = "*.html"]
#[include = "*.svg"]
#[include = "assets/*"]
struct FrontendAssets;

async fn static_asset(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    serve_embedded_file(path).await
}

async fn serve_index() -> impl IntoResponse {
    serve_embedded_file("index.html").await
}

async fn serve_embedded_file(path: &str) -> Response {
    match FrontendAssets::get(path) {
        Some(content) => {
            let mime_type = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(
                    header::CONTENT_TYPE,
                    HeaderValue::from_str(mime_type.as_ref()).unwrap(),
                )
                .body(Body::from(content.data))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("404 Not Found"))
            .unwrap(),
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing (logging)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // --- Dependency Injection Setup ---
    // Create instances of our in-memory adapters
    // Use Arc for shared ownership across handlers/requests
    let user_repo: Arc<dyn Repository<User>> = Arc::new(InMemoryEventRepository::<User>::default());
    let tenant_repo: Arc<dyn Repository<Tenant>> =
        Arc::new(InMemoryEventRepository::<Tenant>::default());
    // Note: Pirep repo needed later
    let event_bus: Arc<dyn EventPublisher> = Arc::new(InMemoryEventBus::default());
    let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::default()); // Added cache

    // Create the application state
    // Clone Arcs for the state - cheap reference count bump
    let app_state = AppState {
        user_repo: user_repo.clone(),
        tenant_repo: tenant_repo.clone(),
        event_bus: event_bus.clone(),
        cache: cache.clone(),
    };

    // Define the Axum router
    let api_routes = Router::new()
        .route("/users", post(handle_register_user_request)) // POST /api/users
        .route("/tenants", post(handle_create_tenant_request)); // POST /api/tenants
    // TODO: Add routes for ChangePassword, GenerateApiKey etc. later

    let app = Router::new()
        .nest("/api", api_routes) // Nest API routes under /api
        // --- Frontend Serving Routes ---
        // Route for static assets (like CSS, JS, images)
        .route("/assets/{*path}", get(static_asset))
        // Route for the main index.html
        .route("/", get(serve_index))
        // Fallback route for SPA (Single Page Application) routing - serves index.html
        // This allows React Router to handle client-side routing
        .fallback(get(serve_index))
        // Add state to the router
        .with_state(app_state);

    // Define the address and port to run the server on
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("API Gateway listening on {}", addr);

    // Create a TCP listener
    let listener = TcpListener::bind(addr).await.unwrap();

    // Run the server
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

// --- Application State ---
// Holds shared dependencies like repositories, event bus, etc.
// Clone is cheap as it clones Arcs (reference counts)
#[derive(Clone)]
#[allow(dead_code)]
struct AppState {
    user_repo: Arc<dyn Repository<User>>,
    tenant_repo: Arc<dyn Repository<Tenant>>,
    // pirep_repo: Arc<dyn Repository<Pirep>>, // Add later
    event_bus: Arc<dyn EventPublisher>,
    cache: Arc<dyn Cache>,
}

// TODO: Implement Axum route handlers (e.g., in application/commands/register_user.rs)
// These handlers will extract the AppState and the request body (command DTO),
// instantiate the appropriate CommandHandler (e.g., RegisterUserHandler),
// passing the dependencies from AppState, and call handler.handle(command).
// They will then map the Result<(), CoreError> to an Axum response (e.g., 201 Created or 400/500 error).
