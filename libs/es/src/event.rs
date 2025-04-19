use std::fmt::Debug;

pub trait DomainEvent: Debug + Clone + PartialEq + Send + Sync {
    /// A name specifying the event, used for event upcasting.
    fn event_type(&self) -> String;
    /// A version of the `event_type`, used for event upcasting.
    fn event_version(&self) -> String;
}
