use std::collections::HashMap;
use std::marker::PhantomData;

use crate::persist::{EventStoreAggregateContext, PersistedEventRepository};
use crate::{Aggregate, AggregateError, Binarize, DomainEvent, EventEnvelope, EventStore};

use super::{PersistenceError, SerializedEvent};

pub struct PersistedEventStore<R, A, B>
where
    R: PersistedEventRepository,
    A: Aggregate + Send + Sync,
    B: Binarize<A::Event>,
{
    repo: R,
    binarizer: B,
    _phantom: PhantomData<A>,
}

impl<R, A, B> PersistedEventStore<R, A, B>
where
    R: PersistedEventRepository,
    A: Aggregate + Send + Sync,
    B: Binarize<A::Event>,
{
    pub fn new_event_store(repo: R, binarizer: B) -> Self {
        Self {
            repo,
            binarizer,
            _phantom: PhantomData,
        }
    }

    fn serialize_events(
        &self,
        events: &[EventEnvelope<A>],
    ) -> Result<Vec<SerializedEvent>, PersistenceError> {
        let mut results = Vec::default();
        for event in events {
            let aggregate_type = A::TYPE.to_string();
            let event_type = event.payload.event_type();
            let event_version = event.payload.event_version();
            let payload = self.binarizer.event_to_bytes(&event.payload)?;
            let metadata = vec![];
            results.push(SerializedEvent {
                aggregate_id: event.aggregate_id.clone(),
                sequence: event.sequence,
                aggregate_type,
                event_type,
                event_version,
                payload,
                metadata,
            })
        }
        Ok(results)
    }

    fn deserialize_events(
        &self,
        events: Vec<SerializedEvent>,
    ) -> Result<Vec<EventEnvelope<A>>, PersistenceError> {
        let mut results = Vec::default();
        for event in events {
            let payload = self.binarizer.event_from_bytes(&event.payload)?;
            let metadata = HashMap::new();
            results.push(EventEnvelope {
                aggregate_id: event.aggregate_id,
                sequence: event.sequence,
                payload,
                metadata,
            })
        }
        Ok(results)
    }

    fn wrap_events(
        aggregate_id: &str,
        last_sequence: usize,
        resultant_events: Vec<A::Event>,
        base_metadata: HashMap<String, String>,
    ) -> Vec<EventEnvelope<A>> {
        resultant_events
            .into_iter()
            .zip(last_sequence + 1..)
            .map(|(payload, sequence)| EventEnvelope {
                aggregate_id: aggregate_id.to_string(),
                sequence,
                payload,
                metadata: base_metadata.clone(),
            })
            .collect()
    }
}

impl<R, A, B> EventStore<A> for PersistedEventStore<R, A, B>
where
    R: PersistedEventRepository,
    A: Aggregate + Send + Sync,
    B: Binarize<A::Event>,
{
    type AC = EventStoreAggregateContext<A>;

    async fn load_events(
        &self,
        aggregate_id: &str,
    ) -> Result<Vec<EventEnvelope<A>>, AggregateError<A::Error>> {
        let serialized_events = self.repo.get_events::<A>(aggregate_id).await?;
        let events = self.deserialize_events(serialized_events.clone())?;
        Ok(events)
    }

    async fn load_aggregate(
        &self,
        aggregate_id: &str,
    ) -> Result<Self::AC, crate::AggregateError<<A as Aggregate>::Error>> {
        let mut context: EventStoreAggregateContext<A> =
            EventStoreAggregateContext::context_for(aggregate_id, true);
        let events_to_apply = self.load_events(aggregate_id).await?;
        for envelope in events_to_apply {
            context.current_sequence = envelope.sequence;
            let event = envelope.payload;
            context.aggregate.apply(event);
        }
        Ok(context)
    }

    async fn commit(
        &self,
        events: Vec<<A as Aggregate>::Event>,
        context: Self::AC,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<Vec<crate::EventEnvelope<A>>, crate::AggregateError<<A as Aggregate>::Error>> {
        let aggregate_id = context.aggregate_id.clone();
        let last_sequence = context.current_sequence;

        let wrapped_events = Self::wrap_events(&aggregate_id, last_sequence, events, metadata);
        let serialized_events: Vec<SerializedEvent> = self.serialize_events(&wrapped_events)?;
        self.repo.persist::<A>(&serialized_events).await?;
        Ok(wrapped_events)
    }
}
