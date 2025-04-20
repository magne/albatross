// Removed unused Argon2 imports
use axum::{
    extract::{Request, State}, // Added State import
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
// Removed unused Cache import
use serde::{Deserialize, Serialize};
// Removed unused Arc import
use tracing::{info, warn}; // Added info

use crate::AppState; // Import AppState

// User context extracted from a valid API key
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub tenant_id: Option<String>,
    // Add roles or other relevant info later
}

/// Middleware to authenticate requests using an API key provided in the Authorization header.
pub async fn api_key_auth(
    State(app_state): State<AppState>, // Inject AppState to access cache
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
                    // Found the key in cache, attempt to deserialize
                    match serde_json::from_slice::<AuthenticatedUser>(&cached_data) {
                        Ok(user_context) => {
                            info!("API Key authenticated for user: {}", user_context.user_id);
                            // Add user context to request extensions
                            req.extensions_mut().insert(user_context);
                            // Proceed to the next middleware/handler
                            Ok(next.run(req).await)
                        }
                        Err(e) => {
                            // Data in cache was corrupted or wrong format
                            warn!(
                                "Failed to deserialize cached user data for provided API key: {}",
                                e
                            );
                            Err(StatusCode::UNAUTHORIZED) // Treat as unauthorized
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
