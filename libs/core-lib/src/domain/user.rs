use crate::{Aggregate, Command, CoreError, Event};
use async_trait::async_trait;
use proto::user::{
    ApiKeyGenerated, ChangePassword, GenerateApiKey, LoginUser, PasswordChanged, RegisterUser,
    Role, UserLoggedIn, UserRegistered,
};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// --- User Aggregate ---

#[derive(Debug, Default, Clone)]
pub struct User {
    id: String,
    version: u64,
    username: String,
    email: String,
    #[allow(dead_code)] // Not read within aggregate; used by external auth logic
    password_hash: String, // Store the hash
    role: Role,
    tenant_id: Option<String>,
    api_keys: HashMap<String, String>, // key_id -> key_hash
                                       // Add other user state fields here (e.g., status)
}

// --- Commands ---

impl Command for RegisterUser {}
impl Command for ChangePassword {}
impl Command for GenerateApiKey {}
impl Command for LoginUser {} // Note: Login might not directly modify aggregate state

// --- Events ---

impl Event for UserRegistered {}
impl Event for PasswordChanged {}
impl Event for ApiKeyGenerated {}
impl Event for UserLoggedIn {} // Note: Login event might be for auditing/projections

// --- Errors ---

#[derive(thiserror::Error, Debug)]
pub enum UserError {
    #[error("Core Error: {0}")]
    Core(#[from] CoreError),
    #[error("User already exists (ID: {0})")]
    AlreadyExists(String),
    #[error("User not found (ID: {0})")]
    NotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Invalid role assignment: {0}")]
    InvalidRole(String),
    #[error("Tenant ID is required for non-PlatformAdmin roles")]
    TenantIdRequired,
    #[error("Invalid password")] // Keep generic for security
    InvalidPassword,
}

// --- Aggregate Implementation ---

#[async_trait]
impl Aggregate for User {
    // Define which commands this aggregate handles directly
    // Note: LoginUser might be handled differently (e.g., by an auth service reading projections)
    // but we include it here if the aggregate needs to emit UserLoggedIn event.
    type Command = UserCommand;
    type Event = UserEvent;
    type Error = UserError;

    fn aggregate_id(&self) -> &str {
        &self.id
    }

    fn version(&self) -> u64 {
        self.version
    }

    /// Apply state changes based on events.
    fn apply(&mut self, event: Self::Event) {
        match event {
            UserEvent::Registered(UserRegistered {
                user_id,
                username,
                email,
                role, // role is i32 here
                tenant_id,
                ..
            }) => {
                self.id = user_id;
                self.username = username;
                self.email = email;
                // Convert i32 from proto to local Role enum
                self.role = Role::try_from(role).unwrap_or(Role::Unspecified);
                self.tenant_id = tenant_id;
                // Password hash is set implicitly during registration event application
                // but not stored directly in the event for security. Assume it's set here.
            }
            UserEvent::PasswordChanged(PasswordChanged { .. }) => {
                // Password hash updated implicitly, not stored in event
            }
            UserEvent::ApiKeyGenerated(ApiKeyGenerated {
                key_id,
                api_key_hash,
                ..
            }) => {
                self.api_keys.insert(key_id, api_key_hash);
            }
            UserEvent::LoggedIn(UserLoggedIn { .. }) => {
                // Login event might not change aggregate state directly
            }
        }
        self.version += 1;
    }

    /// Handle commands and produce events.
    async fn handle(&self, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        // Basic validation: Ensure aggregate exists for commands other than Register
        if self.version == 0 && !matches!(command, UserCommand::Register(_)) {
            return Err(UserError::NotFound(
                // Attempt to get ID from command if possible, otherwise generic message
                match &command {
                    UserCommand::ChangePassword(c) => c.user_id.clone(),
                    UserCommand::GenerateApiKey(c) => c.user_id.clone(),
                    // LoginUser doesn't have user_id directly
                    _ => "unknown".to_string(),
                },
            ));
        }

        match command {
            UserCommand::Register(cmd) => self.handle_register(cmd).await,
            UserCommand::ChangePassword(cmd) => self.handle_change_password(cmd).await,
            UserCommand::GenerateApiKey(cmd) => self.handle_generate_api_key(cmd).await,
            UserCommand::Login(cmd) => self.handle_login(cmd).await,
        }
    }
}

impl User {
    // --- Command Handlers ---

    async fn handle_register(&self, command: RegisterUser) -> Result<Vec<UserEvent>, UserError> {
        if self.version > 0 {
            return Err(UserError::AlreadyExists(self.id.clone()));
        }
        // Input validation
        if command.user_id.is_empty()
            || command.username.is_empty()
            || command.email.is_empty()
            || command.password_hash.is_empty()
        {
            return Err(UserError::InvalidInput("Missing required fields".into()));
        }
        // Use generated CamelCase variant names
        let role = Role::try_from(command.initial_role)
            .map_err(|_| UserError::InvalidRole("Invalid role value".into()))?;
        if role != Role::PlatformAdmin && command.tenant_id.is_none() {
            return Err(UserError::TenantIdRequired);
        }
        if role == Role::PlatformAdmin && command.tenant_id.is_some() {
            return Err(UserError::InvalidRole(
                "PlatformAdmin cannot belong to a tenant".into(),
            ));
        }

        let timestamp = generate_timestamp();
        let event = UserRegistered {
            user_id: command.user_id,
            username: command.username,
            email: command.email,
            // Password hash is NOT included in the event
            role: command.initial_role, // Keep as i32 in event
            tenant_id: command.tenant_id,
            timestamp,
        };

        // Need to apply password hash state change implicitly here, as it's not in the event
        // This is a slight deviation from pure ES, often handled by command handler logic
        // setting state *before* returning the event, or having a separate internal state update.
        // For now, assume the projection or command handler applies the hash.

        Ok(vec![UserEvent::Registered(event)])
    }

    async fn handle_change_password(
        &self,
        command: ChangePassword,
    ) -> Result<Vec<UserEvent>, UserError> {
        if command.user_id != self.id {
            return Err(UserError::NotFound(command.user_id)); // Or Unauthorized
        }
        if command.new_password_hash.is_empty() {
            return Err(UserError::InvalidInput(
                "New password hash cannot be empty".into(),
            ));
        }

        let timestamp = generate_timestamp();
        let event = PasswordChanged {
            user_id: command.user_id,
            timestamp,
            // New hash is NOT included in the event
        };
        // Implicit state update of self.password_hash needed here or by caller

        Ok(vec![UserEvent::PasswordChanged(event)])
    }

    async fn handle_generate_api_key(
        &self,
        command: GenerateApiKey,
    ) -> Result<Vec<UserEvent>, UserError> {
        if command.user_id != self.id {
            return Err(UserError::NotFound(command.user_id)); // Or Unauthorized
        }

        // Generate a new key ID and hash (actual key generation happens in command handler/service)
        // This is simplified here. Real implementation needs secure random generation & hashing.
        let key_id = format!("key_{}", self.api_keys.len() + 1);
        let api_key_hash = format!("hash_for_{}", key_id); // Placeholder hash

        let timestamp = generate_timestamp();
        let event = ApiKeyGenerated {
            user_id: command.user_id,
            key_id,
            key_name: command.key_name,
            api_key_hash, // Store the hash
            timestamp,
        };

        Ok(vec![UserEvent::ApiKeyGenerated(event)])
    }

    async fn handle_login(
        &self,
        _command: LoginUser, // Prefixed with underscore as it's currently unused in this placeholder
    ) -> Result<Vec<UserEvent>, UserError> {
        // !!! SECURITY WARNING !!!
        // Aggregates should typically NOT handle plain text passwords.
        // Password verification should happen *before* calling the aggregate,
        // likely in the command handler or an authentication service, comparing
        // the attempt against the hash read from a projection.
        //
        // This handler is included conceptually but should be revisited.
        // It might only emit UserLoggedIn if verification *outside* the aggregate succeeded.

        // Placeholder: Assume verification happened elsewhere.
        // if !password_verify(&command.password_attempt, &self.password_hash) {
        //     return Err(UserError::InvalidPassword);
        // }

        let timestamp = generate_timestamp();
        let event = UserLoggedIn {
            user_id: self.id.clone(), // Need the user ID
            timestamp,
        };

        Ok(vec![UserEvent::LoggedIn(event)])
    }
}

// Helper function for timestamps
fn generate_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

// --- Command/Event Enums (Needed for Aggregate trait implementation) ---
// These wrap the specific Protobuf types.

#[derive(Debug, Clone)]
pub enum UserCommand {
    Register(RegisterUser),
    ChangePassword(ChangePassword),
    GenerateApiKey(GenerateApiKey),
    Login(LoginUser),
}

// Implement the marker trait for the enum
impl Command for UserCommand {}

#[derive(Debug, Clone)]
pub enum UserEvent {
    Registered(UserRegistered),
    PasswordChanged(PasswordChanged),
    ApiKeyGenerated(ApiKeyGenerated),
    LoggedIn(UserLoggedIn),
}

// Implement the marker trait for the enum
impl Event for UserEvent {}

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Aggregate;
    use proto::user::Role;

    #[tokio::test]
    async fn test_register_user() {
        let aggregate = User::default();
        let command = UserCommand::Register(RegisterUser {
            user_id: "user-1".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            initial_role: Role::Pilot as i32,
            tenant_id: Some("tenant-1".to_string()),
        });

        let result = aggregate.handle(command).await;
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        match &events[0] {
            UserEvent::Registered(UserRegistered {
                user_id,
                username,
                email,
                role,
                tenant_id,
                ..
            }) => {
                assert_eq!(user_id, "user-1");
                assert_eq!(username, "testuser");
                assert_eq!(email, "test@example.com");
                assert_eq!(*role, Role::Pilot as i32); // Compare with i32 value
                assert_eq!(tenant_id.as_deref(), Some("tenant-1"));
            }
            _ => panic!("Expected UserRegistered event"),
        }
    }

    #[tokio::test]
    async fn test_register_user_already_exists() {
        let mut aggregate = User::default();
        aggregate.apply(UserEvent::Registered(UserRegistered {
            user_id: "user-1".to_string(),
            username: "existing".to_string(),
            email: "exist@example.com".to_string(),
            role: Role::Pilot as i32,
            tenant_id: Some("tenant-1".to_string()),
            timestamp: "0".to_string(),
        }));
        // Implicitly set password hash during apply if needed for state checks
        aggregate.password_hash = "existing_hash".to_string();

        let command = UserCommand::Register(RegisterUser {
            user_id: "user-2".to_string(), // Different ID, but aggregate exists
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            password_hash: "new_hash".to_string(),
            initial_role: Role::Pilot as i32,
            tenant_id: Some("tenant-1".to_string()),
        });

        let result = aggregate.handle(command).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            UserError::AlreadyExists(id) => assert_eq!(id, "user-1"),
            _ => panic!("Expected AlreadyExists error"),
        }
    }

    #[tokio::test]
    async fn test_register_user_missing_tenant_id() {
        let aggregate = User::default();
        let command = UserCommand::Register(RegisterUser {
            user_id: "user-1".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            initial_role: Role::Pilot as i32,
            tenant_id: None,
        });

        let result = aggregate.handle(command).await;
        assert!(result.is_err());
        assert!(matches!(result.err().unwrap(), UserError::TenantIdRequired));
    }

    #[tokio::test]
    async fn test_register_platform_admin_with_tenant_id() {
        let aggregate = User::default();
        let command = UserCommand::Register(RegisterUser {
            user_id: "admin-1".to_string(),
            username: "adminuser".to_string(),
            email: "admin@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            initial_role: Role::PlatformAdmin as i32,
            tenant_id: Some("tenant-1".to_string()), // PlatformAdmin should NOT have tenant_id
        });

        let result = aggregate.handle(command).await;
        assert!(result.is_err());
        match result.err().unwrap() {
            UserError::InvalidRole(msg) => assert!(msg.contains("PlatformAdmin cannot belong")),
            _ => panic!("Expected InvalidRole error"),
        }
    }

    #[tokio::test]
    async fn test_change_password() {
        let mut aggregate = User::default();
        aggregate.apply(UserEvent::Registered(UserRegistered {
            user_id: "user-1".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            role: Role::Pilot as i32,
            tenant_id: Some("tenant-1".to_string()),
            timestamp: "0".to_string(),
        }));
        aggregate.password_hash = "old_hash".to_string(); // Set initial hash state

        let command = UserCommand::ChangePassword(ChangePassword {
            user_id: "user-1".to_string(),
            new_password_hash: "new_hashed_password".to_string(),
        });

        let result = aggregate.handle(command).await;
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        match &events[0] {
            UserEvent::PasswordChanged(PasswordChanged { user_id, .. }) => {
                assert_eq!(user_id, "user-1");
            }
            _ => panic!("Expected PasswordChanged event"),
        }
        // In a real scenario, the command handler would update the hash state
        // aggregate.password_hash = "new_hashed_password".to_string();
        // assert_eq!(aggregate.password_hash, "new_hashed_password");
    }

    #[tokio::test]
    async fn test_generate_api_key() {
        let mut aggregate = User::default();
        aggregate.apply(UserEvent::Registered(UserRegistered {
            user_id: "user-1".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            role: Role::Pilot as i32,
            tenant_id: Some("tenant-1".to_string()),
            timestamp: "0".to_string(),
        }));

        let command = UserCommand::GenerateApiKey(GenerateApiKey {
            user_id: "user-1".to_string(),
            key_name: "Test Key".to_string(),
        });

        let result = aggregate.handle(command).await;
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let key_id_generated: String;
        match &events[0] {
            UserEvent::ApiKeyGenerated(ApiKeyGenerated {
                user_id,
                key_id,
                key_name,
                api_key_hash,
                ..
            }) => {
                assert_eq!(user_id, "user-1");
                assert_eq!(key_name, "Test Key");
                assert!(!key_id.is_empty());
                assert!(!api_key_hash.is_empty());
                key_id_generated = key_id.clone(); // Save for apply check
            }
            _ => panic!("Expected ApiKeyGenerated event"),
        }

        // Test apply
        aggregate.apply(events[0].clone());
        assert_eq!(aggregate.version(), 2); // Registered + ApiKeyGenerated
        assert!(aggregate.api_keys.contains_key(&key_id_generated));
    }

    // Note: Test for Login command is omitted as verification logic is outside aggregate
}
