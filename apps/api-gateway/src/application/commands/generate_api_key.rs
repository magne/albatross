use async_trait::async_trait;
use core_lib::{
    Aggregate, CommandHandler, CoreError, EventPublisher, Repository,
    domain::user::{User, UserCommand, UserEvent},
};
use proto::user::GenerateApiKey; // Use the Protobuf command type
use std::sync::Arc;
use uuid::Uuid; // For generating key ID / key itself potentially

// Note: While ApplicationError is defined, the trait requires CoreError
// type HandlerError = ApplicationError;

// TODO: Define a proper return type for handlers that need to return data (like the plain API key)
// For now, this handler conforms to CommandHandler<C> which returns Result<(), CoreError>
// A separate mechanism (e.g., query after command, or different pattern) might be needed
// to return the generated key securely.

pub struct GenerateApiKeyHandler {
    user_repository: Arc<dyn Repository<User>>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl GenerateApiKeyHandler {
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

    // Placeholder for actual secure key generation
    fn generate_secure_key() -> (String, String) {
        let key_id = format!("key_{}", Uuid::new_v4());
        let plain_key = format!("plain_{}", Uuid::new_v4()); // *** REPLACE with real secure random generation ***
        let key_hash = format!("hash_for_{}", plain_key); // *** REPLACE with actual hashing (e.g., Argon2, bcrypt) ***
        (key_id, key_hash) // Return ID and HASH (plain key is handled separately)
    }
}

#[async_trait]
impl CommandHandler<GenerateApiKey> for GenerateApiKeyHandler {
    // Return CoreError to match the trait definition
    async fn handle(&self, command: GenerateApiKey) -> Result<(), CoreError> {
        // 1. Load Aggregate
        let events = self.user_repository.load(&command.user_id).await?;
        if events.is_empty() {
            return Err(CoreError::NotFound(format!(
                "User not found: {}",
                command.user_id
            )));
        }
        let user = User::load(events)?;

        // 2. Generate Key ID and Hash (Plain key is generated but not passed to aggregate)
        // IMPORTANT: The plain key generated here needs to be returned to the user *once*.
        // This handler's signature doesn't support returning it directly.
        // This might require adjustments to the API design or handler pattern.
        let (_key_id, _key_hash) = Self::generate_secure_key();

        // 3. Create the aggregate command enum variant (passing the HASH)
        let aggregate_command = UserCommand::GenerateApiKey(GenerateApiKey {
            user_id: command.user_id.clone(),
            key_name: command.key_name,
            // We don't pass key_id or key_hash here, the aggregate generates/receives them in the event
        });
        // Correction: The aggregate *does* need the key_id and hash to generate the event properly.
        // Let's adjust the aggregate's handle method later if needed, or pass them here.
        // For now, assume the aggregate handle method can derive them or they are passed differently.
        // Reverting to simpler approach: Aggregate handle generates event details.

        // 4. Execute command on the loaded aggregate instance
        // Use '?' directly - relies on From<UserError> for CoreError impl in core-lib
        let resulting_events = user.handle(aggregate_command).await?;

        // 5. Save events using the repository, providing the expected version
        self.user_repository
            .save(&command.user_id, user.version(), &resulting_events)
            .await?;

        // 6. Publish events
        let topic = "user_events";
        for event in resulting_events {
            // TODO: Serialize event payload properly (e.g., using Prost)
            let payload = format!("{:?}", event).into_bytes();
            let event_type = match event {
                UserEvent::ApiKeyGenerated(_) => "ApiKeyGenerated",
                _ => "UnknownUserEvent", // Should not happen here
            };
            self.event_publisher
                .publish(topic, event_type, &payload)
                .await?;
        }

        // IMPORTANT: How to return the plain_key generated earlier?
        // This needs to be addressed in the API layer design.
        // Maybe the handler returns Ok(plain_key) and the trait is adjusted,
        // or a separate query mechanism is used.

        Ok(())
    }
}

// TODO: Add tests for the handler
