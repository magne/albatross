use async_trait::async_trait;
use std::error::Error as StdError;

// Declare modules
pub mod adapters;
pub mod domain;

// Define a common error type for the core library
#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("Aggregate not found: {0}")]
    NotFound(String),
    #[error("Command validation failed: {0}")]
    Validation(String),
    #[error("Concurrency conflict: Expected version {expected}, found {actual}")]
    Concurrency { expected: u64, actual: u64 },
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Deserialization error: {0}")]
    Deserialization(String),
    #[error("Infrastructure error: {0}")]
    Infrastructure(#[from] Box<dyn StdError + Send + Sync>),
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

// Implement From<UserError> for CoreError (and potentially others later)
// This allows '?' to convert specific aggregate errors into CoreError in handlers
impl From<domain::user::UserError> for CoreError {
    fn from(err: domain::user::UserError) -> Self {
        match err {
            domain::user::UserError::Core(ce) => ce,
            domain::user::UserError::NotFound(id) => CoreError::NotFound(id),
            domain::user::UserError::AlreadyExists(id) => {
                CoreError::Validation(format!("User already exists: {}", id))
            }
            domain::user::UserError::InvalidInput(msg)
            | domain::user::UserError::InvalidRole(msg) => CoreError::Validation(msg),
            domain::user::UserError::TenantIdRequired => {
                CoreError::Validation("Tenant ID required".into())
            }
            domain::user::UserError::InvalidPassword => {
                CoreError::Unauthorized("Invalid password".into())
            } // Map to Unauthorized
        }
    }
}

// Implement From<TenantError> for CoreError
impl From<domain::tenant::TenantError> for CoreError {
    fn from(err: domain::tenant::TenantError) -> Self {
        match err {
            domain::tenant::TenantError::Core(ce) => ce,
            domain::tenant::TenantError::AlreadyExists(id) => {
                CoreError::Validation(format!("Tenant already exists: {}", id))
            }
            domain::tenant::TenantError::InvalidInput(msg) => CoreError::Validation(msg),
        }
    }
}

// Implement From<PirepError> for CoreError
impl From<domain::pirep::PirepError> for CoreError {
    fn from(err: domain::pirep::PirepError) -> Self {
        match err {
            domain::pirep::PirepError::Core(ce) => ce,
            domain::pirep::PirepError::AlreadyExists(id) => {
                CoreError::Validation(format!("PIREP already submitted: {}", id))
            }
            domain::pirep::PirepError::InvalidInput(msg) => CoreError::Validation(msg),
        }
    }
}

// Marker trait for commands
pub trait Command: Send + Sync + 'static {}

// Marker trait for events (consider adding methods like event_id, event_name later)
// Needs to be serializable/deserializable (likely via Protobuf)
pub trait Event: Send + Sync + 'static {
    // fn event_type(&self) -> &'static str;
    // fn event_version(&self) -> u16;
}

// Trait for aggregate roots
#[async_trait]
pub trait Aggregate: Send + Sync + Default {
    type Command: Command;
    type Event: Event;
    type Error: From<CoreError>; // Aggregates define their specific error type

    fn aggregate_id(&self) -> &str;
    fn version(&self) -> u64;

    /// Apply an event to mutate the aggregate's state.
    /// This should not fail if the event is valid for the current state.
    fn apply(&mut self, event: Self::Event);

    /// Handle a command and produce events.
    /// This is where business logic and validation occur.
    async fn handle(&self, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error>;

    /// Load the aggregate state from a sequence of events.
    fn load(events: Vec<Self::Event>) -> Result<Self, Self::Error> {
        let mut aggregate = Self::default();
        for event in events {
            aggregate.apply(event);
        }
        Ok(aggregate)
    }
}

// Port for interacting with the event store
#[async_trait]
pub trait Repository<A: Aggregate>: Send + Sync {
    /// Load events for a specific aggregate instance.
    async fn load(&self, aggregate_id: &str) -> Result<Vec<A::Event>, CoreError>;

    /// Save new events for an aggregate instance, handling concurrency.
    async fn save(
        &self,
        aggregate_id: &str,
        expected_version: u64,
        events: &[A::Event],
    ) -> Result<(), CoreError>;
}

// Port for handling commands
#[async_trait]
pub trait CommandHandler<C: Command>: Send + Sync {
    async fn handle(&self, command: C) -> Result<(), CoreError>;
}

// Port for handling events (e.g., in projections)
#[async_trait]
pub trait EventHandler<E: Event>: Send + Sync {
    async fn handle(&self, event: &E) -> Result<(), CoreError>;
}

// Port for publishing events to a message bus
#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(
        &self,
        topic: &str,
        event_type: &str,
        event_payload: &[u8],
    ) -> Result<(), CoreError>;
    // Consider a batch publish method later
}

// Port for subscribing to events from a message bus
// Actual subscription mechanism is adapter-specific. This might evolve.
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    // Placeholder: The adapter implementation will handle the subscription loop.
    // Maybe add methods to register handlers?
    // async fn subscribe<E: Event, H: EventHandler<E>>(&self, topic: &str, handler: H) -> Result<(), CoreError>;
}

// Port for caching data
#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CoreError>;
    async fn set(&self, key: &str, value: &[u8], ttl_seconds: Option<u64>)
    -> Result<(), CoreError>;
    async fn delete(&self, key: &str) -> Result<(), CoreError>;
}
