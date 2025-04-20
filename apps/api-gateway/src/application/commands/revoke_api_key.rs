use core_lib::Cache; // Added Cache import
use core_lib::{
    Aggregate, CommandHandler, CoreError, EventPublisher, Repository,
    domain::user::{User, UserCommand, UserEvent},
};
use cqrs_es::DomainEvent;
use prost::Message;
use proto::user::{
    ApiKeyGenerated, ApiKeyRevoked, PasswordChanged, RevokeApiKey, UserLoggedIn, UserRegistered,
};
use std::sync::Arc;
use tracing;

pub struct RevokeApiKeyHandler {
    user_repository: Arc<dyn Repository>,
    event_publisher: Arc<dyn EventPublisher>,
    cache: Arc<dyn Cache>, // Added cache field
}

impl RevokeApiKeyHandler {
    #[allow(dead_code)]
    pub fn new(
        user_repository: Arc<dyn Repository>,
        event_publisher: Arc<dyn EventPublisher>,
        cache: Arc<dyn Cache>, // Added cache parameter
    ) -> Self {
        Self {
            user_repository,
            event_publisher,
            cache, // Store cache
        }
    }
}

// Implement CommandHandler for the RevokeApiKey proto message
impl CommandHandler<RevokeApiKey> for RevokeApiKeyHandler {
    async fn handle(&self, command: RevokeApiKey) -> Result<(), CoreError> {
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
                "ApiKeyRevoked" => match ApiKeyRevoked::decode(stored_event.payload.as_slice()) {
                    Ok(e) => user.apply(UserEvent::ApiKeyRevoked(e)),
                    Err(err) => {
                        return Err(CoreError::Deserialization(format!(
                            "Failed to decode ApiKeyRevoked: {}",
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

        // 2. Create the aggregate command enum variant
        let aggregate_command = UserCommand::RevokeApiKey(RevokeApiKey {
            user_id: command.user_id.clone(),
            key_id: command.key_id.clone(),
        });

        // 3. Execute command on the loaded aggregate instance
        // Use '?' directly - relies on From<UserError> for CoreError impl in core-lib
        let resulting_events: Vec<UserEvent> = user.handle(aggregate_command).await?;

        // --- Serialize Events for Saving ---
        let events_to_save: Vec<(String, Vec<u8>)> = resulting_events
            .iter()
            .map(|event| {
                let payload = match event {
                    UserEvent::Registered(e) => e.encode_to_vec(),
                    UserEvent::PasswordChanged(e) => e.encode_to_vec(),
                    UserEvent::ApiKeyGenerated(e) => e.encode_to_vec(),
                    UserEvent::ApiKeyRevoked(e) => e.encode_to_vec(),
                    UserEvent::LoggedIn(e) => e.encode_to_vec(),
                };
                (event.event_type(), payload)
            })
            .collect();
        // --- End Serialization ---

        // 4. Save events using the repository, providing the expected version
        self.user_repository
            .save(&command.user_id, user.version(), &events_to_save)
            .await?;

        // 5. Publish events (using the serialized data)
        let topic = "user_events";
        for (event_type, payload) in &events_to_save {
            self.event_publisher
                .publish(topic, event_type, payload)
                .await?;
        }

        // --- Cache Invalidation ---
        // After successfully saving/publishing the event, attempt to remove the key from cache.
        let key_id_cache_key = format!("keyid_{}", command.key_id);
        let plain_key_result = self.cache.get(&key_id_cache_key).await;

        match plain_key_result {
            Ok(Some(plain_key_bytes)) => {
                // Successfully retrieved the plain key bytes associated with the key_id.
                // Now attempt to delete both cache entries.

                // 1. Delete the key_id -> plain_key mapping
                match self.cache.delete(&key_id_cache_key).await {
                    Ok(_) => tracing::info!(
                        "Deleted key_id mapping from cache (key: {})",
                        key_id_cache_key
                    ),
                    Err(e) => tracing::warn!(
                        "Failed to delete key_id mapping from cache (key: {}): {}. Manual cleanup might be needed.",
                        key_id_cache_key,
                        e
                    ),
                }

                // 2. Decode plain_key and delete the plain_key -> AuthenticatedUser mapping
                match String::from_utf8(plain_key_bytes) {
                    Ok(plain_key) => match self.cache.delete(&plain_key).await {
                        Ok(_) => tracing::info!(
                            "Deleted API key auth data from cache (key: {})",
                            plain_key
                        ),
                        Err(e) => tracing::warn!(
                            "Failed to delete API key auth data from cache (key: {}): {}. Manual cleanup might be needed.",
                            plain_key,
                            e
                        ),
                    },
                    Err(e) => {
                        // This is more serious, as we can't delete the main auth entry without the key.
                        tracing::error!(
                            "Failed to decode plain_key from cache for key_id {}: {}. Stale auth data may remain until TTL.",
                            command.key_id,
                            e
                        );
                    }
                }
            }
            Ok(None) => {
                // Key_id mapping wasn't found. It might have been deleted already, expired, or never existed.
                tracing::warn!(
                    "Key_id mapping not found in cache during revocation (key: {}). Assuming already invalid/removed.",
                    key_id_cache_key
                );
                // We cannot delete the main auth entry without the plain_key.
            }
            Err(e) => {
                // Failed to *read* from the cache to get the plain_key.
                tracing::error!(
                    "Cache error looking up key_id mapping for revocation (key: {}): {}. Cache invalidation skipped.",
                    key_id_cache_key,
                    e
                );
                // Cannot proceed with cache deletion.
            }
        }
        // --- End Cache Invalidation ---

        Ok(()) // Return success as the aggregate state was updated.
    }
}

// TODO: Add tests for the handler, including cache interaction
