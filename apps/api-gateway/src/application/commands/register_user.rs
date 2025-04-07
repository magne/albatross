use crate::AppState;
use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse}; // Axum imports
use core_lib::{
    Aggregate,
    CommandHandler,
    CoreError,
    EventPublisher,
    Repository, // CommandHandler trait is needed
    domain::user::{User, UserCommand, UserError, UserEvent},
};
use proto::user::RegisterUser; // Import Role
use serde::Deserialize; // For request DTO
use std::sync::Arc;
use uuid::Uuid; // Need Uuid for generating ID // Import AppState from main.rs (or wherever it's defined - assuming main.rs for now)

// Define a specific error type for this handler/application layer
// Note: While ApplicationError is defined, the trait requires CoreError
// type HandlerError = ApplicationError;

pub struct RegisterUserHandler {
    // Use Arc<dyn Trait> for dependency injection
    user_repository: Arc<dyn Repository<User>>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl RegisterUserHandler {
    #[allow(dead_code)]
    pub fn new(
        user_repository: Arc<dyn Repository<User>>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            user_repository,
            event_publisher,
        }
    }
}

#[async_trait]
impl CommandHandler<RegisterUser> for RegisterUserHandler {
    // Return CoreError to match the trait definition
    async fn handle(&self, command: RegisterUser) -> Result<(), CoreError> {
        // 1. Load Aggregate (or check if exists - Register should fail if exists)
        // For Register, we expect the aggregate *not* to exist yet.
        // We could load to check, but the aggregate's handle method already does this.

        // 2. Create the aggregate command enum variant
        // Note: Password hashing should happen *here* before passing to aggregate.
        // Placeholder for hashing:
        let password_hash = format!("hashed_{}", command.password_hash); // Replace with actual hashing

        let aggregate_command = UserCommand::Register(RegisterUser {
            user_id: command.user_id.clone(), // Ensure ID is passed
            username: command.username,
            email: command.email,
            password_hash, // Pass the hash
            initial_role: command.initial_role,
            tenant_id: command.tenant_id,
        });

        // 3. Execute command on a *default* aggregate instance for creation commands
        let default_user = User::default();
        let events = default_user.handle(aggregate_command).await.map_err(|e| {
            // Map UserError to CoreError for the return type
            match e {
                UserError::Core(ce) => ce,
                UserError::AlreadyExists(id) => {
                    CoreError::Validation(format!("User already exists: {}", id))
                }
                UserError::InvalidInput(msg) | UserError::InvalidRole(msg) => {
                    CoreError::Validation(msg)
                }
                UserError::TenantIdRequired => CoreError::Validation("Tenant ID required".into()),
                UserError::NotFound(_) => {
                    CoreError::Internal("NotFound error during registration".into())
                } // Should not happen
                UserError::InvalidPassword => {
                    CoreError::Internal("InvalidPassword error during registration".into())
                } // Should not happen
            }
        })?;

        // 4. Save events using the repository
        // For creation, expected_version is 0
        self.user_repository
            .save(&command.user_id, 0, &events)
            .await?; // This returns CoreError, which converts via #[from]

        // 5. Publish events
        // TODO: Define proper topic naming convention
        let topic = "user_events";
        for event in events {
            // TODO: Serialize event payload properly (e.g., using Prost)
            // Placeholder: using debug format
            let payload = format!("{:?}", event).into_bytes();
            let event_type = match event {
                UserEvent::Registered(_) => "UserRegistered",
                _ => "UnknownUserEvent", // Should not happen here
            };
            self.event_publisher
                .publish(topic, event_type, &payload)
                .await?; // This returns CoreError, which converts via #[from]
        }

        Ok(())
    }
}

// TODO: Add tests for the handler, mocking the repository and publisher

// --- DTO for the HTTP Request ---

#[derive(Deserialize, Debug)]
pub struct RegisterUserDto {
    username: String,
    email: String,
    password_plaintext: String, // Receive plain text password from API
    initial_role: i32,          // Receive role as i32
    tenant_id: Option<String>,
}

// --- Axum Route Handler ---

pub async fn handle_register_user_request(
    State(state): State<AppState>,        // Extract AppState
    Json(payload): Json<RegisterUserDto>, // Extract JSON payload
) -> impl IntoResponse {
    // 1. Generate User ID
    let user_id = Uuid::new_v4().to_string();

    // 2. Hash Password (Placeholder - use a proper hashing library like argon2/bcrypt)
    // IMPORTANT: Hashing should be computationally intensive. DO NOT do it directly on the request thread in production.
    // Consider offloading to a blocking task: tokio::task::spawn_blocking(...)
    let password_hash = format!("hashed_{}", payload.password_plaintext); // Replace with actual hashing

    // 3. Create the Protobuf Command
    let command = RegisterUser {
        user_id: user_id.clone(), // Clone ID for command
        username: payload.username,
        email: payload.email,
        password_hash,
        initial_role: payload.initial_role,
        tenant_id: payload.tenant_id,
    };

    // 4. Instantiate the handler
    // Dependencies are cloned (Arc clones) - cheap
    let handler = RegisterUserHandler::new(state.user_repo.clone(), state.event_bus.clone());

    // 5. Execute the command
    match handler.handle(command).await {
        Ok(_) => {
            // Return 201 Created on success
            (
                StatusCode::CREATED,
                Json(serde_json::json!({"user_id": user_id})),
            )
                .into_response()
        }
        Err(e) => {
            // Map CoreError to HTTP status codes
            // TODO: Improve error mapping and response body
            let status_code = match e {
                CoreError::NotFound(_) => StatusCode::NOT_FOUND,
                CoreError::Validation(_) => StatusCode::BAD_REQUEST,
                CoreError::Concurrency { .. } => StatusCode::CONFLICT,
                CoreError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
                _ => StatusCode::INTERNAL_SERVER_ERROR, // Default internal error
            };
            (
                status_code,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}
