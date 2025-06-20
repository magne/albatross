use axum::{
    Router,
    body::Body,
    extract::{Extension, Json, Path, State},
    middleware::{self},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use core_lib::{
    Cache,
    CommandHandler, // Keep CommandHandler if used by other handlers moved here
    CoreError,
    EventPublisher,
    Repository,
    // Import specific adapters if needed for AppState construction in tests,
    // but prefer keeping concrete types out of lib.rs if possible.
};
use http::{HeaderValue, StatusCode, Uri, header};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info}; // Keep tracing

// Re-export or declare modules needed by public items
pub mod application; // Make application module public
use application::{
    commands::{
        create_tenant::handle_create_tenant_request, // Keep if needed by create_app
        generate_api_key::{GenerateApiKeyHandler, GenerateApiKeyInput},
        register_user::handle_register_user_request, // Keep if needed by create_app
        revoke_api_key::RevokeApiKeyHandler,
    },
    middleware::{AuthenticatedUser, api_key_auth},
};
use proto::user::RevokeApiKey;

// --- Public Structs ---

#[derive(RustEmbed)]
#[folder = "../web-ui/dist/"]
#[include = "*.html"]
#[include = "*.svg"]
#[include = "assets/*"]
struct FrontendAssets; // Keep private if only used internally by handlers

#[derive(Deserialize)]
pub struct GenerateApiKeyRequest {
    // Make pub if needed directly by tests (e.g. for request body)
    pub key_name: String,
}

#[derive(serde::Serialize, Deserialize, Debug, Clone)] // Add Deserialize, Debug, Clone for test assertions
pub struct GenerateApiKeyResponse {
    pub key_id: String,
    pub api_key: String,
}

// Holds shared dependencies
#[derive(Clone)]
pub struct AppState {
    // Make pub
    // Use pub fields if tests need to construct it directly, otherwise keep private
    // and provide a constructor if needed. Let's make them pub for simplicity now.
    pub user_repo: Arc<dyn Repository>,
    pub tenant_repo: Arc<dyn Repository>,
    pub event_bus: Arc<dyn EventPublisher>,
    pub cache: Arc<dyn Cache>,
}

// --- Public Functions ---

// Function to create the main Axum router with state
pub fn create_app(app_state: AppState) -> Router {
    // Make pub
    let api_routes = Router::new()
        .route("/users", post(handle_register_user_request)) // Assumes handler is pub or in scope
        .route("/tenants", post(handle_create_tenant_request)) // Assumes handler is pub or in scope
        .route(
            "/users/{user_id}/apikeys",            // Use {} syntax for path parameters
            post(handle_generate_api_key_request), // Make handler pub
        )
        .route(
            "/users/{user_id}/apikeys/{key_id}", // Use {} syntax for path parameters
            delete(handle_revoke_api_key_request), // Make handler pub
        )
        .route(
            "/protected",
            get(protected_route).route_layer(middleware::from_fn_with_state(
                // Make handler pub
                app_state.clone(),
                api_key_auth, // Assumes middleware is pub or in scope
            )),
        );

    Router::new()
        .nest("/api", api_routes)
        .route("/assets/{*path}", get(static_asset)) // Make handler pub
        .route("/", get(serve_index)) // Make handler pub
        .fallback(get(serve_index)) // Use same handler
        .with_state(app_state)
}

// --- Public Route Handlers & Helpers (moved from main.rs) ---

pub async fn static_asset(uri: Uri) -> impl IntoResponse {
    // Make pub
    let path = uri.path().trim_start_matches('/');
    serve_embedded_file(path).await
}

pub async fn serve_index() -> impl IntoResponse {
    // Make pub
    serve_embedded_file("index.html").await
}

// Keep private if only used internally
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

pub async fn handle_generate_api_key_request(
    // Make pub
    State(app_state): State<AppState>,
    Path(user_id): Path<String>,
    Json(payload): Json<GenerateApiKeyRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let handler = GenerateApiKeyHandler::new(
        app_state.user_repo.clone(),
        app_state.event_bus.clone(),
        app_state.cache.clone(), // Pass the cache from AppState
    );
    let input = GenerateApiKeyInput {
        user_id,
        key_name: payload.key_name,
    };

    match handler.handle(input).await {
        Ok((key_id, plain_key)) => Ok(Json(GenerateApiKeyResponse {
            key_id,
            api_key: plain_key,
        })),
        Err(e) => Err(map_core_error(e)), // Make map_core_error pub
    }
}

pub async fn handle_revoke_api_key_request(
    // Make pub
    State(app_state): State<AppState>,
    Path((user_id, key_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let handler = RevokeApiKeyHandler::new(
        app_state.user_repo.clone(),
        app_state.event_bus.clone(),
        app_state.cache.clone(), // Pass the cache from AppState
    );
    let command = RevokeApiKey { user_id, key_id };

    match handler.handle(command).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(map_core_error(e)), // Make map_core_error pub
    }
}

pub async fn protected_route(Extension(user): Extension<AuthenticatedUser>) -> impl IntoResponse {
    // Make pub
    info!("Access granted for user: {:?}", user);
    (StatusCode::OK, format!("Hello, user {}", user.user_id))
}

// Make pub so handlers can use it
pub fn map_core_error(err: CoreError) -> StatusCode {
    error!("CoreError occurred: {:?}", err);
    match err {
        CoreError::NotFound(_) => StatusCode::NOT_FOUND,
        CoreError::Validation(_) => StatusCode::BAD_REQUEST,
        CoreError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        CoreError::Concurrency { .. } => StatusCode::CONFLICT,
        CoreError::Internal(_)
        | CoreError::Serialization(_)
        | CoreError::Deserialization(_)
        | CoreError::Infrastructure(_)
        | CoreError::Configuration(_) => StatusCode::INTERNAL_SERVER_ERROR,
        CoreError::AlreadyExists(_) => StatusCode::CONFLICT,
    }
}
