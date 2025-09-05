use crate::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use core_lib::{
    CommandHandler, CoreError, EventPublisher, Repository,
    domain::user::{User, UserCommand, UserEvent},
};
use cqrs_es::{Aggregate, DomainEvent};
use prost::Message;
use proto::user::{LoginUser, UserLoggedIn};
use argon2::password_hash::PasswordVerifier;
use serde::Deserialize;
use sqlx::Row;
use std::sync::Arc;
use tracing::{error, info};

pub struct LoginHandler {
    user_repository: Arc<dyn Repository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl LoginHandler {
    pub fn new(
        user_repository: Arc<dyn Repository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            user_repository,
            event_publisher,
        }
    }
}

impl CommandHandler<LoginUser> for LoginHandler {
    async fn handle(&self, command: LoginUser) -> Result<(), CoreError> {
        // For now, create a demo user_id from username
        // In production, you'd look up the user by username first
        let user_id = format!("user-{}", command.username);

        // Load the user aggregate
        let stored_events = self.user_repository.load(&user_id).await?;
        let mut user = User::default();
        for stored_event in stored_events {
            match stored_event.event_type.as_str() {
                "UserRegistered" => {
                    if let Ok(e) = proto::user::UserRegistered::decode(stored_event.payload.as_slice()) {
                        user.apply(UserEvent::Registered(e))
                    }
                }
                "PasswordChanged" => {
                    if let Ok(e) = proto::user::PasswordChanged::decode(stored_event.payload.as_slice()) {
                        user.apply(UserEvent::PasswordChanged(e))
                    }
                }
                "ApiKeyGenerated" => {
                    if let Ok(e) = proto::user::ApiKeyGenerated::decode(stored_event.payload.as_slice()) {
                        user.apply(UserEvent::ApiKeyGenerated(e))
                    }
                }
                "UserLoggedIn" => {
                    if let Ok(e) = UserLoggedIn::decode(stored_event.payload.as_slice()) {
                        user.apply(UserEvent::LoggedIn(e))
                    }
                }
                "ApiKeyRevoked" => {
                    if let Ok(e) = proto::user::ApiKeyRevoked::decode(stored_event.payload.as_slice()) {
                        user.apply(UserEvent::ApiKeyRevoked(e))
                    }
                }
                _ => {}
            }
        }

        // Execute login command
        let aggregate_command = UserCommand::Login(LoginUser {
            username: command.username.clone(),
            password_attempt: command.password_attempt,
        });

        let events: Vec<UserEvent> = user
            .handle(aggregate_command, &())
            .await
            .map_err(|e| match e {
                core_lib::domain::user::UserError::Core(ce) => ce,
                core_lib::domain::user::UserError::InvalidPassword => {
                    CoreError::Unauthorized("Invalid credentials".into())
                }
                _ => CoreError::Validation("Login failed".into()),
            })?;

        // Serialize and save events
        let events_to_save: Vec<(String, Vec<u8>)> = events
            .iter()
            .map(|event| {
                let payload = match event {
                    UserEvent::LoggedIn(e) => e.encode_to_vec(),
                    _ => Vec::new(),
                };
                (event.event_type(), payload)
            })
            .collect();

        self.user_repository
            .save(&user_id, user.version(), &events_to_save)
            .await?;

        // Publish events
        for (event_type, payload) in &events_to_save {
            let topic = format!("user.{}", user_id);
            self.event_publisher
                .publish(&topic, event_type, payload)
                .await?;
        }

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String, // TODO: Use for actual authentication
}

#[derive(serde::Serialize)]
pub struct LoginResponse {
    api_key: String,
    user_id: String,
}

pub async fn handle_login_request(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Validate input
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Login requires PostgreSQL for user lookup and authentication
    let pool = state.pg_pool.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    info!("Login attempt for user: {}", payload.username);
    info!("Password provided: {}", if payload.password.is_empty() { "empty" } else { "provided" });

    // Look up user by username in the database
    let user_row = sqlx::query(
        r#"
        SELECT user_id, password_hash, role, tenant_id
        FROM users
        WHERE username = $1
        "#,
    )
    .bind(&payload.username)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        error!("Database error during user lookup: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Check if user exists
    let (user_id, stored_hash, role, tenant_id) = match user_row {
        Some(row) => {
            let user_id: String = row.try_get("user_id").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let password_hash: String = row.try_get("password_hash").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let role: String = row.try_get("role").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let tenant_id: Option<String> = row.try_get("tenant_id").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            (user_id, password_hash, role, tenant_id)
        }
        None => {
            // User not found - return generic error for security
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Verify password using argon2
    let password_valid = argon2::Argon2::default()
        .verify_password(payload.password.as_bytes(), &argon2::PasswordHash::new(&stored_hash).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
        .is_ok();

    if !password_valid {
        // Wrong password - return generic error for security
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Password is valid, generate API key
    let api_key = format!("api-key-{}", uuid::Uuid::new_v4());

    // Store API key in cache with user context
    let user_context = serde_json::json!({
        "user_id": user_id,
        "role": role,
        "tenant_id": tenant_id
    });

    if let Err(e) = state.cache.set(&api_key, user_context.to_string().as_bytes(), None).await {
        error!("Failed to cache API key: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let response = LoginResponse {
        api_key,
        user_id,
    };

    Ok(Json(response))
}
