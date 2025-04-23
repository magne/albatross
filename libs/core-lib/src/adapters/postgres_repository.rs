use crate::{CoreError, Repository};
use async_trait::async_trait;
use cqrs_es::persist::SerializedEvent;
use sqlx::{PgPool, Row};

// Define a structure to represent stored events matching DB schema
#[derive(sqlx::FromRow, Debug)]
struct EventRow {
    sequence: i64,
    event_type: String,
    payload: Vec<u8>,
}

/// PostgreSQL implementation of the Repository port using sqlx.
#[derive(Debug, Clone)]
pub struct PostgresEventRepository {
    // No longer generic
    pool: PgPool,
}

impl PostgresEventRepository {
    #[allow(dead_code)]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Repository for PostgresEventRepository {
    // Implement non-generic trait
    /// Load events for a specific aggregate instance.
    async fn load(&self, aggregate_id: &str) -> Result<Vec<SerializedEvent>, CoreError> {
        let table_name = "events"; // Placeholder
        let query = format!(
            "SELECT sequence, event_type, payload FROM {} WHERE aggregate_id = $1 ORDER BY sequence ASC",
            table_name
        );

        let rows: Vec<EventRow> = sqlx::query_as(&query)
            .bind(aggregate_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;

        let events = rows
            .into_iter()
            .map(|row| {
                SerializedEvent::new(
                    aggregate_id.to_string(),
                    row.sequence as usize,
                    "".to_string(),
                    row.event_type,
                    "".to_string(),
                    row.payload,
                    vec![],
                )
            })
            .collect();
        Ok(events)
    }

    /// Save new events for an aggregate instance, handling concurrency.
    async fn save(
        &self,
        aggregate_id: &str,
        expected_version: usize,
        events: &[(String, Vec<u8>)], // Takes (event_type, payload) tuples
    ) -> Result<(), CoreError> {
        if events.is_empty() {
            return Ok(());
        }

        let table_name = "events"; // Placeholder
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;

        // 1. Get current version
        let current_version_query = format!(
            "SELECT MAX(sequence) FROM {} WHERE aggregate_id = $1",
            table_name
        );
        let current_version_row = sqlx::query(&current_version_query)
            .bind(aggregate_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;

        let current_version: usize = match current_version_row {
            Some(row) => row
                .try_get::<Option<i64>, _>(0)
                .map_err(|e| CoreError::Infrastructure(Box::new(e)))?
                .map(|v| v as usize)
                .unwrap_or(0),
            None => 0,
        };

        // 2. Optimistic concurrency check
        if current_version != expected_version {
            tx.rollback()
                .await
                .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;
            return Err(CoreError::Concurrency {
                expected: expected_version,
                actual: current_version,
            });
        }

        // 3. Insert new events
        let mut next_sequence = current_version + 1;
        for (event_type, payload) in events {
            let insert_query = format!(
                "INSERT INTO {} (aggregate_id, sequence, event_type, payload) VALUES ($1, $2, $3, $4)",
                table_name
            );
            sqlx::query(&insert_query)
                .bind(aggregate_id)
                .bind(next_sequence as i64)
                .bind(event_type)
                .bind(payload)
                .execute(&mut *tx)
                .await
                .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;
            next_sequence += 1;
        }

        // 4. Commit transaction
        tx.commit()
            .await
            .map_err(|e| CoreError::Infrastructure(Box::new(e)))?;
        Ok(())
    }
}

// --- Integration Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::{User, UserCommand, UserEvent};
    use cqrs_es::Aggregate;
    use prost::Message;
    use proto::user::{PasswordChanged, RegisterUser, UserRegistered};
    use sqlx::postgres::PgPoolOptions;
    use sqlx::types::Uuid;
    use testcontainers::runners::AsyncRunner;
    use testcontainers::ContainerAsync;
    use testcontainers_modules::postgres::Postgres as PostgresImage;

    // Helper to set up DB
    async fn setup_db() -> (PgPool, ContainerAsync<PostgresImage>) {
        // Use the default image provided by the module
        let node = PostgresImage::default()
            .start()
            .await
            .expect("Failed to start Postgres container");
        let port = node
            .get_host_port_ipv4(5432)
            .await
            .expect("Failed to get host port");
        let connection_string = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&connection_string)
            .await
            .expect("Failed to connect to testcontainer Postgres");

        // Manually create the events table
        sqlx::query(
            "CREATE TABLE events (
                id SERIAL PRIMARY KEY,
                aggregate_id VARCHAR(36) NOT NULL,
                sequence BIGINT NOT NULL,
                event_type VARCHAR(255) NOT NULL,
                payload BYTEA NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE (aggregate_id, sequence)
            );",
        )
        .execute(&pool)
        .await
        .expect("Failed to create events table");
        (pool, node)
    }

    // Helper to serialize events for saving
    fn serialize_events(events: &[UserEvent]) -> Vec<(String, Vec<u8>)> {
        events
            .iter()
            .map(|event| {
                let event_type = match event {
                    UserEvent::Registered(_) => "UserRegistered",
                    UserEvent::PasswordChanged(_) => "PasswordChanged",
                    UserEvent::ApiKeyGenerated(_) => "ApiKeyGenerated",
                    UserEvent::ApiKeyRevoked(_) => "ApiKeyRevoked", // Added
                    UserEvent::LoggedIn(_) => "UserLoggedIn",
                }
                .to_string();
                let payload = match event {
                    UserEvent::Registered(e) => e.encode_to_vec(),
                    UserEvent::PasswordChanged(e) => e.encode_to_vec(),
                    UserEvent::ApiKeyGenerated(e) => e.encode_to_vec(),
                    UserEvent::ApiKeyRevoked(e) => e.encode_to_vec(), // Added
                    UserEvent::LoggedIn(e) => e.encode_to_vec(),
                };
                (event_type, payload)
            })
            .collect()
    }

    #[tokio::test]
    async fn test_save_and_load_postgres() {
        let (pool, _node) = setup_db().await;
        let repo = PostgresEventRepository::new(pool.clone());

        let user_id = Uuid::new_v4().to_string();
        // --- Test Setup: Generate events using the aggregate ---
        let command = UserCommand::Register(RegisterUser {
            user_id: user_id.clone(),
            username: "test-pg".to_string(),
            email: "pg@test.com".to_string(),
            password_hash: "hashed".to_string(),
            initial_role: proto::user::Role::Pilot as i32,
            // Provide a dummy tenant_id for non-admin user registration
            tenant_id: Some("tenant-pg-test".to_string()),
        });
        let default_user = User::default();
        let events_domain = default_user
            .handle(command, &())
            .await
            .expect("Handle failed");
        // --- End Test Setup ---

        let events_to_save = serialize_events(&events_domain);

        // Save events (version 0)
        let save_result = repo.save(&user_id, 0, &events_to_save).await;
        assert!(save_result.is_ok(), "Save failed: {:?}", save_result.err());

        // Load events (returns StoredEventData)
        let loaded_stored_events = repo.load(&user_id).await.expect("Load failed");

        assert_eq!(events_to_save.len(), loaded_stored_events.len());
        assert_eq!(loaded_stored_events[0].sequence, 1);
        assert_eq!(loaded_stored_events[0].event_type, events_to_save[0].0);
        assert_eq!(loaded_stored_events[0].payload, events_to_save[0].1);

        // Optional: Deserialize and compare specific fields
        let deserialized_event = UserRegistered::decode(loaded_stored_events[0].payload.as_slice())
            .expect("Decode failed");
        assert_eq!(deserialized_event.user_id, user_id);
        assert_eq!(deserialized_event.username, "test-pg");

        // Verify DB content
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events WHERE aggregate_id = $1")
            .bind(&user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, events_to_save.len() as i64);
    }

    #[tokio::test]
    async fn test_concurrency_error_postgres() {
        let (pool, _node) = setup_db().await;
        let repo = PostgresEventRepository::new(pool);

        let user_id = Uuid::new_v4().to_string();
        // --- Test Setup: Generate events ---
        let event1_domain = UserEvent::Registered(UserRegistered {
            user_id: user_id.clone(),
            username: "concurrent".to_string(),
            email: "con@test.com".to_string(),
            role: proto::user::Role::Pilot as i32,
            tenant_id: None,
            timestamp: "now".to_string(),
        });
        let event2_domain = UserEvent::PasswordChanged(PasswordChanged {
            user_id: user_id.clone(),
            timestamp: "later".to_string(),
        });
        // --- End Test Setup ---

        let events_to_save1 = serialize_events(&[event1_domain]);
        let events_to_save2 = serialize_events(&[event2_domain]);

        // Save initial event (version 0 -> 1)
        let save_result1 = repo.save(&user_id, 0, &events_to_save1).await;
        assert!(save_result1.is_ok());

        // Try to save another event with wrong expected version (0 instead of 1)
        let save_result2 = repo.save(&user_id, 0, &events_to_save2).await;

        assert!(save_result2.is_err());
        match save_result2.err().unwrap() {
            CoreError::Concurrency { expected, actual } => {
                assert_eq!(expected, 0);
                assert_eq!(actual, 1);
            }
            e => panic!("Expected Concurrency error, got {:?}", e),
        }
    }
}
