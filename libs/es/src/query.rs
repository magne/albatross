use async_trait::async_trait;

use crate::{Aggregate, EventEnvelope};

/// Each CQRS platform should have one or more queries where it will distribute committed
/// events.
///
/// Some example of tasks that queries commonly provide:
/// - update materialized views
/// - publish events to messaging service
/// - trigger a command on another aggregate
#[async_trait]
pub trait Query<A: Aggregate>: Send + Sync {
    /// Events will be dispatched here immediately after being committed.
    async fn dispatch(&self, aggregate_id: &str, events: &[EventEnvelope<A>]);
}
