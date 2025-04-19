/// A serialized version of an event with metadata.
/// Used by repositories to store and load events from a database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedEvent {
    /// The id of the aggregate instance.
    pub aggregate_id: String,
    /// The sequence number of the event for this aggregate instance.
    pub sequence: usize,
    /// The type of aggregate the event applies to.
    pub aggregate_type: String,
    /// The type of event that is serialized.
    pub event_type: String,
    /// The version of the event that is serialized.
    pub event_version: String,
    /// The serialized domain event.
    pub payload: Vec<u8>,
    /// Additional metadata, serialized from a HashMap<String,String>.
    pub metadata: Vec<u8>,
}

impl SerializedEvent {
    /// Create a new [`SerializedEvent`] with the given values.
    pub fn new(
        aggregate_id: String,
        sequence: usize,
        aggregate_type: String,
        event_type: String,
        event_version: String,
        payload: Vec<u8>,
        metadata: Vec<u8>,
    ) -> Self {
        Self {
            aggregate_id,
            sequence,
            aggregate_type,
            event_type,
            event_version,
            payload,
            metadata,
        }
    }
}
