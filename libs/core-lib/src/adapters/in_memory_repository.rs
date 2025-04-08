use crate::{CoreError, Repository, StoredEventData}; // Removed Aggregate
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

/// In-memory implementation of the Repository port for testing and single-executable mode.
/// Stores events associated with their aggregate ID and current version.
#[derive(Debug, Clone, Default)]
pub struct InMemoryEventRepository {
    // Store: Aggregate ID -> (Current Version, Vec<StoredEventData>)
    store: Arc<DashMap<String, (u64, Vec<StoredEventData>)>>,
}

#[async_trait]
impl Repository for InMemoryEventRepository {
    /// Load events for a specific aggregate instance.
    async fn load(&self, aggregate_id: &str) -> Result<Vec<StoredEventData>, CoreError> {
        match self.store.get(aggregate_id) {
            Some(entry) => Ok(entry.value().1.clone()), // Clone the StoredEventData vector
            None => Ok(Vec::new()),
        }
    }

    /// Save new events for an aggregate instance, handling concurrency.
    async fn save(
        &self,
        aggregate_id: &str,
        expected_version: u64,
        events: &[(String, Vec<u8>)], // Takes (event_type, payload) tuples
    ) -> Result<(), CoreError> {
        if events.is_empty() {
            return Ok(());
        }

        let mut entry = self
            .store
            .entry(aggregate_id.to_string())
            .or_insert_with(|| (0, Vec::new()));

        let (current_version, existing_events) = entry.value_mut();

        // Optimistic concurrency check
        if *current_version != expected_version {
            return Err(CoreError::Concurrency {
                expected: expected_version,
                actual: *current_version,
            });
        }

        // Append new events and update version
        let mut next_sequence = *current_version + 1;
        for (event_type, payload) in events {
            existing_events.push(StoredEventData {
                sequence: next_sequence,
                event_type: event_type.clone(),
                payload: payload.clone(),
            });
            next_sequence += 1;
        }
        *current_version = next_sequence - 1; // Update version to the last sequence number used

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::{User, UserCommand, UserEvent};
    use crate::Aggregate;
    use prost::Message;
    use proto::user::{PasswordChanged, RegisterUser, UserRegistered};

    // Helper to serialize events for saving (same as in postgres tests)
    fn serialize_events(events: &[UserEvent]) -> Vec<(String, Vec<u8>)> {
        events
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
            .collect()
    }

    #[tokio::test]
    async fn test_save_and_load_new_aggregate() {
        let repo = InMemoryEventRepository::default(); // No generic type needed
        let aggregate_id = "agg-1".to_string();

        // --- Test Setup: Generate events using the aggregate ---
        let command = UserCommand::Register(RegisterUser {
            user_id: aggregate_id.clone(),
            username: "test-inmem".to_string(),
            email: "inmem@test.com".to_string(),
            password_hash: "hashed".to_string(),
            initial_role: proto::user::Role::Pilot as i32,
            // Provide a dummy tenant_id for non-admin user registration
            tenant_id: Some("tenant-inmem-test".to_string()),
        });
        let default_user = User::default();
        let events_domain = default_user.handle(command).await.expect("Handle failed");
        // --- End Test Setup ---

        let events_to_save = serialize_events(&events_domain);

        let result = repo.save(&aggregate_id, 0, &events_to_save).await;
        assert!(result.is_ok());

        let loaded_events = repo.load(&aggregate_id).await.unwrap();
        assert_eq!(loaded_events.len(), 1);
        assert_eq!(loaded_events[0].sequence, 1);
        assert_eq!(loaded_events[0].event_type, events_to_save[0].0);
        assert_eq!(loaded_events[0].payload, events_to_save[0].1);

        // Check internal state
        let entry = repo.store.get(&aggregate_id).unwrap();
        assert_eq!(entry.value().0, 1); // Version should be 1
    }

    #[tokio::test]
    async fn test_append_events() {
        let repo = InMemoryEventRepository::default(); // No generic type needed
        let aggregate_id = "agg-2".to_string();

        // --- Setup initial event ---
        let event1_domain = UserEvent::Registered(UserRegistered {
            user_id: aggregate_id.clone(),
            username: "append-user".to_string(),
            email: "append@test.com".to_string(),
            role: proto::user::Role::Pilot as i32,
            tenant_id: None,
            timestamp: "0".to_string(),
        });
        let events_to_save1 = serialize_events(&[event1_domain]);
        repo.save(&aggregate_id, 0, &events_to_save1).await.unwrap();
        // --- End setup ---

        // --- Setup second event ---
        let event2_domain = UserEvent::PasswordChanged(PasswordChanged {
            user_id: aggregate_id.clone(),
            timestamp: "1".to_string(),
        });
        let events_to_save2 = serialize_events(&[event2_domain]);
        // --- End setup ---

        let result = repo.save(&aggregate_id, 1, &events_to_save2).await; // Expected version is 1
        assert!(result.is_ok());

        let loaded_events = repo.load(&aggregate_id).await.unwrap();
        assert_eq!(loaded_events.len(), 2);
        assert_eq!(loaded_events[0].sequence, 1);
        assert_eq!(loaded_events[1].sequence, 2);
        assert_eq!(loaded_events[1].event_type, events_to_save2[0].0);
        assert_eq!(loaded_events[1].payload, events_to_save2[0].1);

        let entry = repo.store.get(&aggregate_id).unwrap();
        assert_eq!(entry.value().0, 2); // Version should be 2
    }

    #[tokio::test]
    async fn test_concurrency_error() {
        let repo = InMemoryEventRepository::default(); // No generic type needed
        let aggregate_id = "agg-3".to_string();
        // --- Setup initial event ---
        let event1_domain = UserEvent::Registered(UserRegistered {
            user_id: aggregate_id.clone(),
            username: "concurr-user".to_string(),
            email: "concurr@test.com".to_string(),
            role: proto::user::Role::Pilot as i32,
            tenant_id: None,
            timestamp: "0".to_string(),
        });
        let events_to_save1 = serialize_events(&[event1_domain]);
        repo.save(&aggregate_id, 0, &events_to_save1).await.unwrap(); // Version is now 1
                                                                      // --- End setup ---

        // --- Setup second event ---
        let event2_domain = UserEvent::PasswordChanged(PasswordChanged {
            user_id: aggregate_id.clone(),
            timestamp: "1".to_string(),
        });
        let events_to_save2 = serialize_events(&[event2_domain]);
        // --- End setup ---

        // Try to save with incorrect expected version (0 instead of 1)
        let result = repo.save(&aggregate_id, 0, &events_to_save2).await;

        assert!(result.is_err());
        match result.err().unwrap() {
            CoreError::Concurrency { expected, actual } => {
                assert_eq!(expected, 0);
                assert_eq!(actual, 1);
            }
            _ => panic!("Expected Concurrency error"),
        }

        // Ensure events were not saved
        let loaded_events = repo.load(&aggregate_id).await.unwrap();
        assert_eq!(loaded_events.len(), 1); // Should only contain the initial event
        let entry = repo.store.get(&aggregate_id).unwrap();
        assert_eq!(entry.value().0, 1); // Version should still be 1
    }

    #[tokio::test]
    async fn test_load_non_existent_aggregate() {
        let repo = InMemoryEventRepository::default();
        let aggregate_id = "agg-non-existent".to_string();

        let loaded_events = repo.load(&aggregate_id).await.unwrap();
        assert!(loaded_events.is_empty());
    }

    #[tokio::test]
    async fn test_save_empty_event_list() {
        let repo = InMemoryEventRepository::default();
        let aggregate_id = "agg-empty".to_string();
        let empty_events: Vec<(String, Vec<u8>)> = vec![];

        let result = repo.save(&aggregate_id, 0, &empty_events).await;
        assert!(result.is_ok());

        // Ensure nothing was actually stored
        assert!(!repo.store.contains_key(&aggregate_id));
    }
}
