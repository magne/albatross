use crate::{Aggregate, CoreError, Repository}; // Removed Event import
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

/// In-memory implementation of the Repository port for testing and single-executable mode.
/// Stores events associated with their aggregate ID and current version.
#[derive(Debug, Clone)]
pub struct InMemoryEventRepository<A: Aggregate>
where
    A::Event: Clone, // Events need to be cloneable to be stored and retrieved
{
    // Store: Aggregate ID -> (Current Version, Vec<Events>)
    store: Arc<DashMap<String, (u64, Vec<A::Event>)>>,
}

impl<A: Aggregate> Default for InMemoryEventRepository<A>
where
    A::Event: Clone,
{
    fn default() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
        }
    }
}

#[async_trait]
impl<A: Aggregate + Send + Sync> Repository<A> for InMemoryEventRepository<A>
where
    A::Event: Clone + Send + Sync, // Ensure Event is Send + Sync for async contexts
{
    /// Load events for a specific aggregate instance.
    async fn load(&self, aggregate_id: &str) -> Result<Vec<A::Event>, CoreError> {
        match self.store.get(aggregate_id) {
            Some(entry) => Ok(entry.value().1.clone()), // Clone the events vector
            None => Ok(Vec::new()), // No events found means it doesn't exist yet
        }
    }

    /// Save new events for an aggregate instance, handling concurrency.
    async fn save(
        &self,
        aggregate_id: &str,
        expected_version: u64,
        events: &[A::Event],
    ) -> Result<(), CoreError> {
        if events.is_empty() {
            return Ok(()); // Nothing to save
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
        existing_events.extend_from_slice(events);
        *current_version += events.len() as u64;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, CoreError, Event}; // Added Event import here as it's used in tests
    use std::sync::atomic::{AtomicU64, Ordering};

    // --- Test Aggregate Setup ---

    #[derive(Debug, Clone, PartialEq)]
    enum TestEvent {
        Created(String),
        Updated(String),
    }
    impl Event for TestEvent {}

    #[derive(Debug, Clone)]
    enum TestCommand {
        Create(String),
        Update(String),
    }
    impl Command for TestCommand {}

    #[derive(Debug, thiserror::Error)]
    enum TestAggregateError {
        #[error("Core Error: {0}")]
        Core(#[from] CoreError),
        #[error("Invalid state for command")]
        InvalidState,
        #[error("Already created")]
        AlreadyCreated,
    }

    #[derive(Debug, Default)]
    struct TestAggregate {
        id: String,
        version: AtomicU64, // Use AtomicU64 for interior mutability if needed, or just u64
        data: Option<String>,
    }

    #[async_trait]
    impl Aggregate for TestAggregate {
        type Command = TestCommand;
        type Event = TestEvent;
        type Error = TestAggregateError;

        fn aggregate_id(&self) -> &str {
            &self.id
        }

        fn version(&self) -> u64 {
            self.version.load(Ordering::Relaxed)
        }

        fn apply(&mut self, event: Self::Event) {
            match event {
                TestEvent::Created(id) => {
                    self.id = id;
                    self.data = Some("initial".to_string());
                }
                TestEvent::Updated(new_data) => {
                    self.data = Some(new_data);
                }
            }
            // Increment version on apply
            self.version.fetch_add(1, Ordering::Relaxed);
        }

        async fn handle(&self, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
            match command {
                TestCommand::Create(id) => {
                    // Check version using self.version() which takes &self
                    if self.version() > 0 {
                        Err(TestAggregateError::AlreadyCreated)
                    } else {
                        Ok(vec![TestEvent::Created(id)])
                    }
                }
                TestCommand::Update(data) => {
                    if self.version() == 0 {
                        Err(TestAggregateError::InvalidState) // Cannot update if not created
                    } else {
                        Ok(vec![TestEvent::Updated(data)])
                    }
                }
            }
        }
    }

    // --- Tests ---

    #[tokio::test]
    async fn test_save_and_load_new_aggregate() {
        let repo = InMemoryEventRepository::<TestAggregate>::default();
        let aggregate_id = "agg-1".to_string();
        // Clone aggregate_id when creating the event to avoid moving it
        let events_to_save = vec![TestEvent::Created(aggregate_id.clone())];

        // Pass aggregate_id as &str
        let result = repo.save(&aggregate_id, 0, &events_to_save).await;
        assert!(result.is_ok());

        let loaded_events = repo.load(&aggregate_id).await.unwrap();
        assert_eq!(loaded_events.len(), 1);
        // Compare with the cloned value inside the event
        assert_eq!(loaded_events[0], events_to_save[0]);

        // Check internal state (optional, requires accessing DashMap directly or via debug)
        // Pass aggregate_id as &str
        let entry = repo.store.get(&aggregate_id).unwrap();
        assert_eq!(entry.value().0, 1); // Version should be 1
    }

    #[tokio::test]
    async fn test_append_events() {
        let repo = InMemoryEventRepository::<TestAggregate>::default();
        let aggregate_id = "agg-2".to_string();
        let initial_event = vec![TestEvent::Created(aggregate_id.clone())];
        repo.save(&aggregate_id, 0, &initial_event).await.unwrap();

        let update_event = vec![TestEvent::Updated("new data".to_string())];
        let result = repo.save(&aggregate_id, 1, &update_event).await; // Expected version is 1
        assert!(result.is_ok());

        let loaded_events = repo.load(&aggregate_id).await.unwrap();
        assert_eq!(loaded_events.len(), 2);
        assert_eq!(loaded_events[0], TestEvent::Created(aggregate_id.clone()));
        assert_eq!(loaded_events[1], TestEvent::Updated("new data".to_string()));

        let entry = repo.store.get(&aggregate_id).unwrap();
        assert_eq!(entry.value().0, 2); // Version should be 2
    }

    #[tokio::test]
    async fn test_concurrency_error() {
        let repo = InMemoryEventRepository::<TestAggregate>::default();
        let aggregate_id = "agg-3".to_string();
        let initial_event = vec![TestEvent::Created(aggregate_id.clone())];
        repo.save(&aggregate_id, 0, &initial_event).await.unwrap(); // Version is now 1

        let update_event = vec![TestEvent::Updated("new data".to_string())];
        // Try to save with incorrect expected version (0 instead of 1)
        let result = repo.save(&aggregate_id, 0, &update_event).await;

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
        let repo = InMemoryEventRepository::<TestAggregate>::default();
        let aggregate_id = "agg-non-existent".to_string();

        let loaded_events = repo.load(&aggregate_id).await.unwrap();
        assert!(loaded_events.is_empty());
    }

    #[tokio::test]
    async fn test_save_empty_event_list() {
        let repo = InMemoryEventRepository::<TestAggregate>::default();
        let aggregate_id = "agg-empty".to_string();
        let empty_events: Vec<TestEvent> = vec![];

        let result = repo.save(&aggregate_id, 0, &empty_events).await;
        assert!(result.is_ok());

        // Ensure nothing was actually stored
        assert!(!repo.store.contains_key(&aggregate_id));
    }
}
