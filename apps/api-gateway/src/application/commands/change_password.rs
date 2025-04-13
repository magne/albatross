use core_lib::{
    Aggregate, CommandHandler, CoreError, DomainEvent, EventPublisher, Repository,
    domain::user::{User, UserCommand, UserEvent},
};
use prost::Message;

use proto::user::{ChangePassword, PasswordChanged};
use std::sync::Arc;
use tracing;

pub struct ChangePasswordHandler {
    user_repository: Arc<dyn Repository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl ChangePasswordHandler {
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

impl CommandHandler<ChangePassword> for ChangePasswordHandler {
    async fn handle(&self, command: ChangePassword) -> Result<(), CoreError> {
        // 1. Load Aggregate
        let events = self.user_repository.load(&command.user_id).await?;
        if events.is_empty() {
            // Use specific error from ApplicationError mapped to CoreError if needed,
            // or just use CoreError::NotFound directly.
            return Err(CoreError::NotFound(format!(
                "User not found: {}",
                command.user_id
            )));
        }
        // --- Correct Aggregate Loading ---
        let mut user = User::default();
        for stored_event in events {
            // Deserialize and apply based on type string
            match stored_event.event_type.as_str() {
                "UserRegistered" => {
                    // Need UserRegistered proto type for decoding
                    match proto::user::UserRegistered::decode(stored_event.payload.as_slice()) {
                        Ok(e) => user.apply(UserEvent::Registered(e)),
                        Err(err) => {
                            return Err(CoreError::Deserialization(format!(
                                "Failed to decode UserRegistered: {}",
                                err
                            )));
                        }
                    }
                }
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
                    // Need ApiKeyGenerated proto type for decoding
                    match proto::user::ApiKeyGenerated::decode(stored_event.payload.as_slice()) {
                        Ok(e) => user.apply(UserEvent::ApiKeyGenerated(e)),
                        Err(err) => {
                            return Err(CoreError::Deserialization(format!(
                                "Failed to decode ApiKeyGenerated: {}",
                                err
                            )));
                        }
                    }
                }
                "UserLoggedIn" => {
                    // Need UserLoggedIn proto type for decoding
                    match proto::user::UserLoggedIn::decode(stored_event.payload.as_slice()) {
                        Ok(e) => user.apply(UserEvent::LoggedIn(e)),
                        Err(err) => {
                            return Err(CoreError::Deserialization(format!(
                                "Failed to decode UserLoggedIn: {}",
                                err
                            )));
                        }
                    }
                }
                _ => {
                    // Log or handle unknown event types if necessary
                    tracing::warn!(
                        "Skipping unknown event type during user load: {}",
                        stored_event.event_type
                    );
                }
            }
        }
        // --- End Aggregate Loading ---

        // 2. Hash the new password (should happen here, before passing to aggregate)
        // Placeholder for hashing:
        let new_password_hash = format!("hashed_{}", command.new_password_hash); // Replace with actual hashing

        // 3. Create the aggregate command enum variant
        let aggregate_command = UserCommand::ChangePassword(ChangePassword {
            user_id: command.user_id.clone(),
            new_password_hash, // Pass the hash
        });

        // 4. Execute command on the loaded aggregate instance
        // Use '?' directly - relies on From<UserError> for CoreError impl in core-lib
        let resulting_events: Vec<UserEvent> = user.handle(aggregate_command).await?; // Type annotation for clarity

        // --- Serialize Events for Saving ---
        let events_to_save: Vec<(String, Vec<u8>)> = resulting_events
            .iter()
            .map(|event| {
                let payload = match event {
                    UserEvent::Registered(e) => e.encode_to_vec(),
                    UserEvent::PasswordChanged(e) => e.encode_to_vec(),
                    UserEvent::ApiKeyGenerated(e) => e.encode_to_vec(),
                    UserEvent::LoggedIn(e) => e.encode_to_vec(),
                };
                (event.event_type(), payload)
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

        Ok(())
    }
}

// TODO: Add tests for the handler
