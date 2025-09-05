use crate::AppState;
use axum::{Json, extract::State, http::{StatusCode, HeaderMap, header}, response::IntoResponse};
use core_lib::{
    CommandHandler, CoreError, EventPublisher, Repository,
    domain::user::{User, UserCommand, UserError, UserEvent},
};
use cqrs_es::{Aggregate, DomainEvent};
use prost::Message;
use proto::user::{RegisterUser, Role as ProtoRole};
use crate::application::authz::{parse_role, AuthRole};
use crate::application::middleware::AuthenticatedUser;
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{rand_core::OsRng, SaltString};
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
        // Password is already hashed by the HTTP handler
        let aggregate_command = UserCommand::Register(RegisterUser {
            user_id: command.user_id.clone(),
            username: command.username,
            email: command.email,
            password_hash: command.password_hash,
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
        for (event_type, payload) in &events_to_save {
            let topic = match event_type.as_str() {
                "UserRegistered" => format!("user.{}", command.user_id),
                _ => "user_events".to_string(),
            };
            self.event_publisher
                .publish(&topic, event_type, payload)
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
    headers: HeaderMap,
    Json(payload): Json<RegisterUserDto>,
) -> impl IntoResponse {
    // Optional contextual auth: derive AuthenticatedUser from Authorization header (if present)
    let auth_header = headers.get(header::AUTHORIZATION).and_then(|h| h.to_str().ok());
    let ctx_opt: Option<AuthenticatedUser> = match auth_header {
        Some(h) if h.starts_with("Bearer ") => {
            let token = h.trim_start_matches("Bearer ").trim();
            match state.cache.get(token).await {
                Ok(Some(bytes)) => serde_json::from_slice::<AuthenticatedUser>(&bytes).ok(),
                _ => None,
            }
        }
        _ => None,
    };

    // RBAC:
    // 1. If no context -> allow ONLY bootstrap PlatformAdmin creation:
    //    - initial_role == PlatformAdmin
    //    - tenant_id is None
    // 2. If context exists:
    //    - PlatformAdmin may register any role (PlatformAdmin must have no tenant_id in payload)
    //    - TenantAdmin may register NON-Platform roles only within its own tenant (payload.tenant_id == ctx.tenant_id, cannot create PlatformAdmin)
    //    - Pilot cannot register anyone (forbidden)
    let requested_role = payload.initial_role;
    let is_platform_admin_role = requested_role == ProtoRole::PlatformAdmin as i32;
    let mut forbidden = false;

    if let Some(ctx) = &ctx_opt {
        // Authorized path
        let role = match parse_role(&ctx.role) {
            Some(r) => r,
            None => return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "invalid role"}))).into_response(),
        };

        match role {
            AuthRole::PlatformAdmin => {
                // PlatformAdmin constraints:
                // If creating another PlatformAdmin, tenant_id must be None
                if is_platform_admin_role && payload.tenant_id.is_some() {
                    return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "PlatformAdmin cannot have tenant_id"}))).into_response();
                }
                // If creating non-Platform role, tenant_id must be Some
                if !is_platform_admin_role && payload.tenant_id.is_none() {
                    return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "tenant_id required for non-PlatformAdmin role"}))).into_response();
                }
            }
            AuthRole::TenantAdmin => {
                // TenantAdmin cannot create PlatformAdmin
                if is_platform_admin_role {
                    forbidden = true;
                } else {
                    // Must supply tenant_id matching its own
                    if let Some(tid) = &payload.tenant_id {
                        if Some(tid) != ctx.tenant_id.as_ref() {
                            forbidden = true;
                        }
                    } else {
                        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "tenant_id required"}))).into_response();
                    }
                }
            }
            AuthRole::Pilot => {
                forbidden = true;
            }
        }

        if forbidden {
            return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "forbidden"}))).into_response();
        }
    } else {
        // Bootstrap path
        if !(is_platform_admin_role && payload.tenant_id.is_none()) {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "authentication required"}))).into_response();
        }
    }
    let user_id = Uuid::new_v4().to_string();

    // Hash password with argon2
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = match argon2.hash_password(payload.password_plaintext.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to hash password"}))).into_response(),
    };

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
