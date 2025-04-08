use async_trait::async_trait;
use core_lib::{
    Aggregate, CommandHandler, CoreError, EventPublisher, Repository,
    domain::user::{User, UserCommand, UserEvent},
};
use prost::Message;
use proto::user::{ApiKeyGenerated, GenerateApiKey, PasswordChanged, UserLoggedIn, UserRegistered};
use std::sync::Arc;
use tracing;
use uuid::Uuid;

// TODO: Define a proper return type for handlers that need to return data (like the plain API key)
// For now, this handler conforms to CommandHandler<C> which returns Result<(), CoreError>
// A separate mechanism (e.g., query after command, or different pattern) might be needed
// to return the generated key securely.

pub struct GenerateApiKeyHandler {
    user_repository: Arc<dyn Repository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl GenerateApiKeyHandler {
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
        let stored_events = self.user_repository.load(&command.user_id).await?;
        if stored_events.is_empty() {
            return Err(CoreError::NotFound(format!(
                "User not found: {}",
                command.user_id
            )));
        }
        // --- Correct Aggregate Loading ---
        let mut user = User::default();
        for stored_event in stored_events {
            // Deserialize and apply based on type string
            match stored_event.event_type.as_str() {
                "UserRegistered" => match UserRegistered::decode(stored_event.payload.as_slice()) {
                    Ok(e) => user.apply(UserEvent::Registered(e)),
                    Err(err) => {
                        return Err(CoreError::Deserialization(format!(
                            "Failed to decode UserRegistered: {}",
                            err
                        )));
                    }
                },
                "PasswordChanged" => {
                    match PasswordChanged::decode(stored_event.payload.as_slice()) {
                        Ok(e) => user.apply(UserEvent::PasswordChanged(e)),
                        Err(err) => {
                            return Err(CoreError::Deserialization(format!(
                                "Failed to decode PasswordChanged: {}",
                                err
                            )));
                        }
                    }
                }
                "ApiKeyGenerated" => {
                    match ApiKeyGenerated::decode(stored_event.payload.as_slice()) {
                        Ok(e) => user.apply(UserEvent::ApiKeyGenerated(e)),
                        Err(err) => {
                            return Err(CoreError::Deserialization(format!(
                                "Failed to decode ApiKeyGenerated: {}",
                                err
                            )));
                        }
                    }
                }
                "UserLoggedIn" => match UserLoggedIn::decode(stored_event.payload.as_slice()) {
                    Ok(e) => user.apply(UserEvent::LoggedIn(e)),
                    Err(err) => {
                        return Err(CoreError::Deserialization(format!(
                            "Failed to decode UserLoggedIn: {}",
                            err
                        )));
                    }
                },
                _ => {
                    tracing::warn!(
                        "Skipping unknown event type during user load: {}",
                        stored_event.event_type
                    );
                }
            }
        }
        // --- End Aggregate Loading ---

        // 2. Generate Key ID and Hash (Plain key is generated but not passed to aggregate)
        let (_key_id, _key_hash) = Self::generate_secure_key();

        // 3. Create the aggregate command enum variant
        let aggregate_command = UserCommand::GenerateApiKey(GenerateApiKey {
            user_id: command.user_id.clone(),
            key_name: command.key_name,
        });

        // 4. Execute command on the loaded aggregate instance
        let resulting_events: Vec<UserEvent> = user.handle(aggregate_command).await?;

        // --- Serialize Events for Saving ---
        let events_to_save: Vec<(String, Vec<u8>)> = resulting_events
            .iter()
            .map(|event| {
                let event_type = match event {
                    UserEvent::Registered(_) => "UserRegistered",
                    UserEvent::PasswordChanged(_) => "PasswordChanged",
                    UserEvent::ApiKeyGenerated(_) => "ApiKeyGenerated",
                    UserEvent::LoggedIn(_) => "UserLoggedIn",
                }
                .to_string();
                let payload = match event {
                    UserEvent::Registered(e) => e.encode_to_vec(),
                    UserEvent::PasswordChanged(e) => e.encode_to_vec(),
                    UserEvent::ApiKeyGenerated(e) => e.encode_to_vec(),
                    UserEvent::LoggedIn(e) => e.encode_to_vec(),
                };
                (event_type, payload)
            })
            .collect();
        // --- End Serialization ---

        // 5. Save events using the repository, providing the expected version
        self.user_repository
            .save(&command.user_id, user.version(), &events_to_save) // Pass serialized data
            .await?;

        // 6. Publish events (using the serialized data)
        let topic = "user_events";
        for (event_type, payload) in &events_to_save {
            // Iterate over serialized data
            self.event_publisher
                .publish(topic, event_type, payload) // Publish raw bytes
                .await?;
        }

        // IMPORTANT: How to return the plain_key generated earlier?
        Ok(())
    }
}

// TODO: Add tests for the handler
