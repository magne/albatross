use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng}, // Removed unused PasswordHash, PasswordVerifier
};
use core_lib::{
    Aggregate,
    CoreError,
    EventPublisher,
    Repository, // Removed unused CommandHandler
    domain::user::{User, UserCommand, UserEvent},
};
use cqrs_es::DomainEvent;
use prost::Message;
use proto::user::{ApiKeyGenerated, GenerateApiKey, PasswordChanged, UserLoggedIn, UserRegistered};
use rand::distr::{Alphanumeric, SampleString}; // Corrected module name again
use rand::rng; // Separate import
use std::sync::Arc;
use tracing;
use uuid::Uuid;

// Input for the GenerateApiKeyHandler
#[derive(Debug, Clone)]
pub struct GenerateApiKeyInput {
    pub user_id: String,
    pub key_name: String,
}

// This handler now returns the plain text key on success
use crate::application::middleware::AuthenticatedUser; // Added AuthenticatedUser import
use core_lib::Cache; // Added Cache import
use serde_json; // Added serde_json import

pub struct GenerateApiKeyHandler {
    user_repository: Arc<dyn Repository>,
    event_publisher: Arc<dyn EventPublisher>,
    cache: Arc<dyn Cache>, // Added cache field
}

impl GenerateApiKeyHandler {
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

    // Generates a secure key, its hash, and a key ID.
    // Returns (key_id, plain_key, key_hash_string) or CoreError
    fn generate_secure_key() -> Result<(String, String, String), CoreError> {
        let key_id = format!("key_{}", Uuid::new_v4());

        // Generate a cryptographically secure random string for the API key
        // Example: 32 characters alphanumeric
        let plain_key = Alphanumeric.sample_string(&mut rng(), 32); // Removed qualification

        // Hash the key using Argon2
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(plain_key.as_bytes(), &salt)
            .map_err(|e| CoreError::Internal(format!("Failed to hash API key: {}", e)))?
            .to_string();

        Ok((key_id, plain_key, password_hash))
    }
}

// Note: This no longer implements CommandHandler directly because the return type changed.
// The calling code (e.g., the Axum route handler) will need to be adjusted.
impl GenerateApiKeyHandler {
    // Returns (key_id, plain_key) on success
    pub async fn handle(&self, input: GenerateApiKeyInput) -> Result<(String, String), CoreError> {
        // 1. Load Aggregate
        let stored_events = self.user_repository.load(&input.user_id).await?;
        if stored_events.is_empty() {
            return Err(CoreError::NotFound(format!(
                "User not found: {}",
                input.user_id
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
                "ApiKeyRevoked" => {
                    // Need ApiKeyRevoked proto type for decoding
                    match proto::user::ApiKeyRevoked::decode(stored_event.payload.as_slice()) {
                        Ok(e) => user.apply(UserEvent::ApiKeyRevoked(e)),
                        Err(err) => {
                            return Err(CoreError::Deserialization(format!(
                                "Failed to decode ApiKeyRevoked: {}",
                                err
                            )));
                        }
                    }
                }
                _ => {
                    tracing::warn!(
                        "Skipping unknown event type during user load: {}",
                        stored_event.event_type
                    );
                }
            }
        }
        // --- End Aggregate Loading ---

        // 2. Generate Key ID, Plain Key, and Hash
        let (key_id, plain_key, key_hash) = Self::generate_secure_key()?;

        // 3. Create the aggregate command enum variant, now including key_id and hash
        let key_hash_for_command = key_hash.clone(); // Clone hash explicitly for command
        let aggregate_command = UserCommand::GenerateApiKey(GenerateApiKey {
            user_id: input.user_id.clone(),     // Use input.user_id
            key_id: key_id.clone(),             // Pass generated key_id
            key_name: input.key_name.clone(),   // Use input.key_name
            api_key_hash: key_hash_for_command, // Pass the clone
        });

        // 4. Execute command on the loaded aggregate instance
        let resulting_events: Vec<UserEvent> = user.handle(aggregate_command).await?;

        // --- Serialize Events for Saving ---
        let events_to_save: Vec<(String, Vec<u8>)> = resulting_events
            .iter()
            .map(|event| {
                let payload = match event {
                    UserEvent::Registered(e) => e.encode_to_vec(),
                    UserEvent::PasswordChanged(e) => e.encode_to_vec(),
                    UserEvent::ApiKeyGenerated(e) => e.encode_to_vec(),
                    UserEvent::ApiKeyRevoked(e) => e.encode_to_vec(), // Added
                    UserEvent::LoggedIn(e) => e.encode_to_vec(),
                };
                (event.event_type(), payload)
            })
            .collect();
        // --- End Serialization ---

        // 5. Save events using the repository, providing the expected version
        self.user_repository
            .save(&input.user_id, user.version(), &events_to_save) // Pass serialized data
            .await?;

        // 6. Publish events (using the serialized data)
        let topic = "user_events";
        for (event_type, payload) in &events_to_save {
            // Iterate over serialized data
            self.event_publisher
                .publish(topic, event_type, payload) // Publish raw bytes
                .await?;
        }

        // 7. Store AuthenticatedUser in cache, keyed by the PLAIN TEXT key
        let authenticated_user = AuthenticatedUser {
            user_id: input.user_id.clone(),
            tenant_id: user.tenant_id().cloned(), // Get tenant_id from loaded user aggregate state
                                                  // key_id: key_id.clone(), // Also store key_id for potential revocation checks later
        };
        let cache_key = plain_key.clone(); // Use the PLAIN TEXT key as the cache key
        let cache_value = serde_json::to_vec(&authenticated_user).map_err(|e| {
            CoreError::Serialization(format!("Failed to serialize AuthenticatedUser: {}", e))
        })?;

        // TODO: Make cache TTL configurable
        let ttl_seconds = Some(3600 * 24 * 30); // e.g., 30 days

        // Store AuthenticatedUser keyed by plain_key
        self.cache
            .set(&cache_key, &cache_value, ttl_seconds)
            .await?;
        // Log only the key_id associated with the cached auth data for security
        tracing::info!("Stored API key auth data in cache (key_id: {})", key_id);

        // Store key_id -> plain_key mapping for revocation
        let key_id_cache_key = format!("keyid_{}", key_id);
        let plain_key_bytes = plain_key.as_bytes().to_vec();
        self.cache
            .set(&key_id_cache_key, &plain_key_bytes, ttl_seconds)
            .await?;
        tracing::info!(
            "Stored API key ID mapping in cache (key: {})",
            key_id_cache_key
        );

        // IMPORTANT: Return both key_id and plain_key
        Ok((key_id, plain_key))
    }
}

// TODO: Add tests for the handler, including cache interaction
