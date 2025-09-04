use axum::{
    Router,
    body::Body,
    extract::{Extension, Json, Path, State},
    middleware::{self},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use tower_http::cors::{CorsLayer, Any};
use core_lib::{
    Cache,
    CommandHandler, // Keep CommandHandler if used by other handlers moved here
    CoreError,
    EventPublisher,
    Repository,
    // Import specific adapters if needed for AppState construction in tests,
    // but prefer keeping concrete types out of lib.rs if possible.
};
use http::{HeaderMap, HeaderValue, StatusCode, Uri, header};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info}; // Keep tracing
use sqlx::PgPool;

// Re-export or declare modules needed by public items
pub mod application; // Make application module public
use application::ws::ws_handler;
use application::{
    commands::{
        change_password::ChangePasswordHandler,
        create_tenant::handle_create_tenant_request, // Keep if needed by create_app
        generate_api_key::{GenerateApiKeyHandler, GenerateApiKeyInput},
        register_user::handle_register_user_request, // Keep if needed by create_app
        revoke_api_key::RevokeApiKeyHandler,
    },
    middleware::{AuthenticatedUser, api_key_auth},
    authz::{authorize, parse_role, Requirement},
    query::{handle_list_tenants, handle_list_users, handle_list_user_api_keys},
};
use cqrs_es::Aggregate;
use prost::Message;
use core_lib::domain::user::{User, UserEvent};
use proto::user::{ApiKeyGenerated, PasswordChanged, UserLoggedIn, UserRegistered, ApiKeyRevoked, RevokeApiKey};

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
    pub user_repo: Arc<dyn Repository>,
    pub tenant_repo: Arc<dyn Repository>,
    pub event_bus: Arc<dyn EventPublisher>,
    pub cache: Arc<dyn Cache>,
    pub pg_pool: Option<PgPool>, // Added optional PgPool for query endpoints
    pub redis_client: Option<redis::Client>, // Redis client for WS pubsub
}

// --- Public Functions ---

// Function to create the main Axum router with state
pub fn create_app(app_state: AppState) -> Router {
    // Make pub
    let api_routes = Router::new()
        // Registration (bootstrap allowed) - POST only
        .route("/users", post(handle_register_user_request))
        // Listing endpoints (auth required) placed on separate list paths to avoid protecting bootstrap registration route
        .route(
            "/users/list",
            get(handle_list_users).route_layer(middleware::from_fn_with_state(
                app_state.clone(),
                api_key_auth,
            )),
        )
        .route(
            "/tenants",
            post(handle_create_tenant_request).route_layer(middleware::from_fn_with_state(
                app_state.clone(),
                api_key_auth,
            )),
        )
        .route(
            "/tenants/list",
            get(handle_list_tenants).route_layer(middleware::from_fn_with_state(
                app_state.clone(),
                api_key_auth,
            )),
        )
        .route(
            "/users/{user_id}/apikeys",            // Use {} syntax for path parameters
            post(handle_generate_api_key_request),
        )
        .route(
            "/users/{user_id}/apikeys/{key_id}", // Use {} syntax for path parameters
            delete(handle_revoke_api_key_request)
                .route_layer(middleware::from_fn_with_state(app_state.clone(), api_key_auth)),
        )
        .route(
            "/users/{user_id}/apikeys/list",
            get(handle_list_user_api_keys).route_layer(middleware::from_fn_with_state(
                app_state.clone(),
                api_key_auth,
            )),
        )
        .route(
            "/users/{user_id}/change-password",
            post(handle_change_password_request).route_layer(middleware::from_fn_with_state(
                app_state.clone(),
                api_key_auth,
            )),
        )
        .route(
            "/protected",
            get(protected_route).route_layer(middleware::from_fn_with_state(
                app_state.clone(),
                api_key_auth,
            )),
        );

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .nest("/api", api_routes)
        .route("/api/ws", get(ws_handler))
        .route("/assets/{*path}", get(static_asset)) // Make handler pub
        .route("/", get(serve_index)) // Make handler pub
        .fallback(get(serve_index)) // Use same handler
        .layer(cors)
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
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(payload): Json<GenerateApiKeyRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Resolve target user aggregate and tenant
    let stored_events = app_state.user_repo.load(&user_id).await.map_err(map_core_error)?;
    let mut target = User::default();
    for stored_event in stored_events {
        match stored_event.event_type.as_str() {
            "UserRegistered" => {
                if let Ok(e) = UserRegistered::decode(stored_event.payload.as_slice()) {
                    target.apply(UserEvent::Registered(e))
                }
            }
            "PasswordChanged" => {
                if let Ok(e) = PasswordChanged::decode(stored_event.payload.as_slice()) {
                    target.apply(UserEvent::PasswordChanged(e))
                }
            }
            "ApiKeyGenerated" => {
                if let Ok(e) = ApiKeyGenerated::decode(stored_event.payload.as_slice()) {
                    target.apply(UserEvent::ApiKeyGenerated(e))
                }
            }
            "UserLoggedIn" => {
                if let Ok(e) = UserLoggedIn::decode(stored_event.payload.as_slice()) {
                    target.apply(UserEvent::LoggedIn(e))
                }
            }
            "ApiKeyRevoked" => {
                if let Ok(e) = ApiKeyRevoked::decode(stored_event.payload.as_slice()) {
                    target.apply(UserEvent::ApiKeyRevoked(e))
                }
            }
            _ => {}
        }
    }
    let target_tenant_id = target.tenant_id().cloned();

    // Attempt to derive context from Authorization header (optional)
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let ctx_opt: Option<AuthenticatedUser> = match auth_header {
        Some(h) if h.starts_with("Bearer ") => {
            let token = h.trim_start_matches("Bearer ").trim();
            match app_state.cache.get(token).await {
                Ok(Some(bytes)) => serde_json::from_slice::<AuthenticatedUser>(&bytes).ok(),
                _ => None,
            }
        }
        _ => None,
    };

    if let Some(ctx) = ctx_opt {
        let role = parse_role(&ctx.role).ok_or(StatusCode::FORBIDDEN)?;
        authorize(
            &ctx.user_id,
            &ctx.tenant_id,
            role,
            Requirement::SelfOrTenantAdmin {
                target_user_id: user_id.clone(),
                target_tenant_id: target_tenant_id.clone(),
            },
        )?;
    } else {
        // Bootstrap: allow unauthenticated creation only if user has no existing keys
        if target.api_key_count() > 0 {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
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
    Extension(ctx): Extension<AuthenticatedUser>,
    Path((user_id, key_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    // Resolve target tenant (self uses ctx; other loads from aggregate)
    let target_tenant_id = if user_id != ctx.user_id {
        let stored_events = app_state.user_repo.load(&user_id).await.map_err(map_core_error)?;
        let mut target = User::default();
        for stored_event in stored_events {
            match stored_event.event_type.as_str() {
                "UserRegistered" => {
                    if let Ok(e) = UserRegistered::decode(stored_event.payload.as_slice()) {
                        target.apply(UserEvent::Registered(e))
                    }
                }
                "PasswordChanged" => {
                    if let Ok(e) = PasswordChanged::decode(stored_event.payload.as_slice()) {
                        target.apply(UserEvent::PasswordChanged(e))
                    }
                }
                "ApiKeyGenerated" => {
                    if let Ok(e) = ApiKeyGenerated::decode(stored_event.payload.as_slice()) {
                        target.apply(UserEvent::ApiKeyGenerated(e))
                    }
                }
                "UserLoggedIn" => {
                    if let Ok(e) = UserLoggedIn::decode(stored_event.payload.as_slice()) {
                        target.apply(UserEvent::LoggedIn(e))
                    }
                }
                "ApiKeyRevoked" => {
                    if let Ok(e) = ApiKeyRevoked::decode(stored_event.payload.as_slice()) {
                        target.apply(UserEvent::ApiKeyRevoked(e))
                    }
                }
                _ => {}
            }
        }
        target.tenant_id().cloned()
    } else {
        ctx.tenant_id.clone()
    };

    let role = parse_role(&ctx.role).ok_or(StatusCode::FORBIDDEN).map_err(|_| StatusCode::FORBIDDEN)?;
    authorize(
        &ctx.user_id,
        &ctx.tenant_id,
        role,
        Requirement::SelfOrTenantAdmin {
            target_user_id: user_id.clone(),
            target_tenant_id,
        },
    ).map_err(|_| StatusCode::FORBIDDEN)?;
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

pub async fn handle_change_password_request(
    State(app_state): State<AppState>,
    Extension(ctx): Extension<AuthenticatedUser>,
    Path(user_id): Path<String>,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<StatusCode, StatusCode> {
    // Only allow users to change their own password
    if ctx.user_id != user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    let handler = ChangePasswordHandler::new(
        app_state.user_repo.clone(),
        app_state.event_bus.clone(),
    );

    let command = proto::user::ChangePassword {
        user_id: user_id.clone(),
        new_password_hash: payload.new_password, // Note: In production, hash this on client side
    };

    match handler.handle(command).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(map_core_error(e)),
    }
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
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
