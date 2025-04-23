use crate::Aggregate;

use crate::persist::{PersistenceError, SerializedEvent};

pub trait PersistedEventRepository: Send + Sync {
    fn get_events<A: Aggregate>(
        &self,
        aggregate_id: &str,
    ) -> impl Future<Output = Result<Vec<SerializedEvent>, PersistenceError>> + Send;

    fn persist<A: Aggregate>(
        &self,
        events: &[SerializedEvent],
    ) -> impl Future<Output = Result<(), PersistenceError>> + Send;
}
