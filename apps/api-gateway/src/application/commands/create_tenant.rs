use crate::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use core_lib::{
    Aggregate, CommandHandler, CoreError, EventPublisher, Repository,
    domain::tenant::{Tenant, TenantError},
};
use prost::Message;
use proto::tenant::CreateTenant;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

pub struct CreateTenantHandler {
    tenant_repository: Arc<dyn Repository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl CreateTenantHandler {
    #[allow(dead_code)]
    pub fn new(
        tenant_repository: Arc<dyn Repository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            tenant_repository,
            event_publisher,
        }
    }
}

impl CommandHandler<CreateTenant> for CreateTenantHandler {
    async fn handle(&self, command: CreateTenant) -> Result<(), CoreError> {
        // 1. Load Aggregate (or check if exists - Create should fail if exists)
        // Similar to RegisterUser, the aggregate's handle method checks for existence.

        // 2. Execute command on a *default* aggregate instance for creation commands
        let default_tenant = Tenant::default();
        let events = default_tenant.handle(command.clone()).await.map_err(|e| {
            // Map TenantError to CoreError for the return type
            match e {
                TenantError::Core(ce) => ce,
                TenantError::AlreadyExists(id) => {
                    CoreError::Validation(format!("Tenant already exists: {}", id))
                }
                TenantError::InvalidInput(msg) => CoreError::Validation(msg),
            }
        })?;

        // --- Serialize Events for Saving ---
        let events_to_save: Vec<(String, Vec<u8>)> = events
            .iter()
            .map(|event| {
                let event_type = "TenantCreated".to_string(); // Only one event type for Tenant
                let payload = event.encode_to_vec();
                (event_type, payload)
            })
            .collect();
        // --- End Serialization ---

        // 3. Save events using the repository
        // For creation, expected_version is 0
        self.tenant_repository
            .save(&command.tenant_id, 0, &events_to_save) // Pass serialized data
            .await?;

        // 4. Publish events (using serialized data)
        // TODO: Define proper topic naming convention
        let topic = "tenant_events";
        for (event_type, payload) in &events_to_save {
            // Iterate over serialized data
            self.event_publisher
                .publish(topic, event_type, payload) // Publish raw bytes
                .await?;
        }

        Ok(())
    }
}

// TODO: Add tests for the handler, mocking the repository and publisher

#[derive(Deserialize, Debug)]
pub struct CreateTenantDto {
    name: String,
    // Add other fields needed from the API request
}

// --- Axum Route Handler ---

pub async fn handle_create_tenant_request(
    State(state): State<AppState>,        // Extract AppState
    Json(payload): Json<CreateTenantDto>, // Extract JSON payload
) -> impl IntoResponse {
    // 1. Generate Tenant ID
    let tenant_id = Uuid::new_v4().to_string();

    // 2. Create the Protobuf Command
    let command = CreateTenant {
        tenant_id: tenant_id.clone(), // Clone ID for command
        name: payload.name,
    };

    // 3. Instantiate the handler
    let handler = CreateTenantHandler::new(state.tenant_repo.clone(), state.event_bus.clone());

    // 4. Execute the command
    match handler.handle(command).await {
        Ok(_) => {
            // Return 201 Created on success
            (
                StatusCode::CREATED,
                Json(serde_json::json!({"tenant_id": tenant_id})),
            )
                .into_response()
        }
        Err(e) => {
            // Map CoreError to HTTP status codes
            // TODO: Improve error mapping and response body
            let status_code = match e {
                CoreError::Validation(_) => StatusCode::BAD_REQUEST,
                CoreError::Concurrency { .. } => StatusCode::CONFLICT, // Should not happen on create
                _ => StatusCode::INTERNAL_SERVER_ERROR,                // Default internal error
            };
            (
                status_code,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}
