use crate::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use core_lib::{
    CommandHandler, CoreError, EventPublisher, Repository,
    domain::user::{User, UserCommand, UserError, UserEvent},
};
use cqrs_es::{Aggregate, DomainEvent};
use prost::Message;
use proto::user::RegisterUser;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

pub struct RegisterUserHandler {
    user_repository: Arc<dyn Repository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl RegisterUserHandler {
    #[allow(dead_code)]
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

impl CommandHandler<RegisterUser> for RegisterUserHandler {
    async fn handle(&self, command: RegisterUser) -> Result<(), CoreError> {
        // 1. Load Aggregate (Not needed for RegisterUser)

        // 2. Create the aggregate command enum variant
        let password_hash = format!("hashed_{}", command.password_hash); // Replace with actual hashing
        let aggregate_command = UserCommand::Register(RegisterUser {
            user_id: command.user_id.clone(),
            username: command.username,
            email: command.email,
            password_hash,
            initial_role: command.initial_role,
            tenant_id: command.tenant_id,
        });

        // 3. Execute command on a *default* aggregate instance
        let default_user = User::default();
        let events: Vec<UserEvent> =
            default_user
                .handle(aggregate_command, &())
                .await
                .map_err(|e| match e {
                    UserError::Core(ce) => ce,
                    UserError::AlreadyExists(id) => {
                        CoreError::Validation(format!("User already exists: {}", id))
                    }
                    UserError::InvalidInput(msg) | UserError::InvalidRole(msg) => {
                        CoreError::Validation(msg)
                    }
                    UserError::TenantIdRequired => {
                        CoreError::Validation("Tenant ID required".into())
                    }
                    _ => CoreError::Internal("Unexpected error during registration".into()),
                })?;

        // --- Serialize Events for Saving ---
        let events_to_save: Vec<(String, Vec<u8>)> = events
            .iter()
            .map(|event| {
                let payload = match event {
                    UserEvent::Registered(e) => e.encode_to_vec(),
                    // Other variants not expected from RegisterUser command
                    _ => Vec::new(), // Or handle error
                };
                (event.event_type(), payload)
            })
            .collect();
        // --- End Serialization ---

        // 4. Save events using the repository
        self.user_repository
            .save(&command.user_id, 0, &events_to_save) // Pass serialized data
            .await?;

        // 5. Publish events (using the serialized data)
        let topic = "user_events";
        for (event_type, payload) in &events_to_save {
            self.event_publisher
                .publish(topic, event_type, payload)
                .await?;
        }

        Ok(())
    }
}

// --- DTO for the HTTP Request ---
#[derive(Deserialize, Debug)]
pub struct RegisterUserDto {
    username: String,
    email: String,
    password_plaintext: String,
    initial_role: i32,
    tenant_id: Option<String>,
}

// --- Axum Route Handler ---
pub async fn handle_register_user_request(
    State(state): State<AppState>,
    Json(payload): Json<RegisterUserDto>,
) -> impl IntoResponse {
    let user_id = Uuid::new_v4().to_string();
    let password_hash = format!("hashed_{}", payload.password_plaintext); // Replace with actual hashing

    let command = RegisterUser {
        user_id: user_id.clone(),
        username: payload.username,
        email: payload.email,
        password_hash,
        initial_role: payload.initial_role,
        tenant_id: payload.tenant_id,
    };

    let handler = RegisterUserHandler::new(state.user_repo.clone(), state.event_bus.clone());

    match handler.handle(command).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(serde_json::json!({"user_id": user_id})),
        )
            .into_response(),
        Err(e) => {
            let status_code = match e {
                CoreError::NotFound(_) => StatusCode::NOT_FOUND,
                CoreError::Validation(_) => StatusCode::BAD_REQUEST,
                CoreError::Concurrency { .. } => StatusCode::CONFLICT,
                CoreError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (
                status_code,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}
