// Removed unused Argon2 imports
use axum::{
    extract::{Request, State}, // Added State import
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
// Removed unused Cache import
use serde::{Deserialize, Serialize};
use prost::Message;
use crate::AppState as GatewayAppState;
use core_lib::domain::user::{User, UserEvent};
use proto::user::{ApiKeyGenerated, ApiKeyRevoked, PasswordChanged, UserLoggedIn, UserRegistered};
use cqrs_es::Aggregate;
// Removed unused Arc import
use tracing::{info, warn}; // Added info


// User context extracted from a valid API key
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub tenant_id: Option<String>,
    pub role: String, // "PlatformAdmin" | "TenantAdmin" | "Pilot"
}

// Backward-compat for legacy cache entries without role
#[derive(Clone, Debug, Serialize, Deserialize)]
struct LegacyAuthenticatedUser {
    pub user_id: String,
    pub tenant_id: Option<String>,
}

/// Middleware to authenticate requests using an API key provided in the Authorization header.
pub async fn api_key_auth(
    State(app_state): State<GatewayAppState>, // Inject AppState to access cache
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let api_key = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            header.strip_prefix("Bearer ").map(|key| key.trim())
        }
        _ => None,
    };

    match api_key {
        Some(key) if !key.is_empty() => {
            // --- Actual Cache-Based Validation ---
            match app_state.cache.get(key).await {
                Ok(Some(cached_data)) => {
                    // Try to deserialize new format
                    match serde_json::from_slice::<AuthenticatedUser>(&cached_data) {
                        Ok(user_context) => {
                            info!("API Key authenticated for user: {}", user_context.user_id);
                            req.extensions_mut().insert(user_context);
                            Ok(next.run(req).await)
                        }
                        Err(_) => {
                            // Attempt to read legacy entry and enrich
                            match serde_json::from_slice::<LegacyAuthenticatedUser>(&cached_data) {
                                Ok(legacy) => {
                                    // Rebuild user aggregate to get role (and tenant, confirm)
                                    let load_res = app_state.user_repo.load(&legacy.user_id).await;
                                    match load_res {
                                        Ok(stored_events) => {
                                            let mut user = User::default();
                                            for stored_event in stored_events {
                                                match stored_event.event_type.as_str() {
                                                    "UserRegistered" => {
                                                        if let Ok(e) = UserRegistered::decode(stored_event.payload.as_slice()) {
                                                            user.apply(UserEvent::Registered(e))
                                                        }
                                                    }
                                                    "PasswordChanged" => {
                                                        if let Ok(e) = PasswordChanged::decode(stored_event.payload.as_slice()) {
                                                            user.apply(UserEvent::PasswordChanged(e))
                                                        }
                                                    }
                                                    "ApiKeyGenerated" => {
                                                        if let Ok(e) = ApiKeyGenerated::decode(stored_event.payload.as_slice()) {
                                                            user.apply(UserEvent::ApiKeyGenerated(e))
                                                        }
                                                    }
                                                    "UserLoggedIn" => {
                                                        if let Ok(e) = UserLoggedIn::decode(stored_event.payload.as_slice()) {
                                                            user.apply(UserEvent::LoggedIn(e))
                                                        }
                                                    }
                                                    "ApiKeyRevoked" => {
                                                        if let Ok(e) = ApiKeyRevoked::decode(stored_event.payload.as_slice()) {
                                                            user.apply(UserEvent::ApiKeyRevoked(e))
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            let role_str = match user.role() {
                                                proto::user::Role::PlatformAdmin => "PlatformAdmin",
                                                proto::user::Role::TenantAdmin => "TenantAdmin",
                                                proto::user::Role::Pilot => "Pilot",
                                                _ => "Unspecified",
                                            }.to_string();
                                            let enriched = AuthenticatedUser {
                                                user_id: legacy.user_id,
                                                tenant_id: user.tenant_id().cloned(),
                                                role: role_str,
                                            };
                                            // Persist enriched entry back to cache with TTL
                                            let ttl_seconds = Some(3600 * 24 * 30);
                                            if let Ok(bytes) = serde_json::to_vec(&enriched)
                                                && let Err(e) = app_state.cache.set(key, &bytes, ttl_seconds).await {
                                                    warn!("Failed to update enriched auth cache entry: {}", e);
                                                }
                                            info!("Enriched legacy auth cache entry with role");
                                            req.extensions_mut().insert(enriched);
                                            Ok(next.run(req).await)
                                        }
                                        Err(e) => {
                                            warn!("Failed to load user for legacy auth enrichment: {}", e);
                                            Err(StatusCode::UNAUTHORIZED)
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to deserialize cached user data for provided API key: {}", e);
                                    Err(StatusCode::UNAUTHORIZED)
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    // Key not found in cache (invalid or revoked)
                    warn!("Provided API Key not found in cache.");
                    Err(StatusCode::UNAUTHORIZED)
                }
                Err(e) => {
                    // Error interacting with the cache
                    warn!("Cache error during API key lookup: {}", e);
                    // Treat as unauthorized, as we cannot confirm validity
                    Err(StatusCode::UNAUTHORIZED)
                }
            }
        }
        _ => {
            warn!(
                "API Key authentication failed: Missing, empty, or invalid Authorization header."
            );
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
