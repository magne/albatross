use crate::persist::PersistenceError;

pub trait Binarize<E>: Send + Sync + 'static {
    fn event_to_bytes(&self, event: &E) -> Result<Vec<u8>, PersistenceError>;
    fn event_from_bytes(&self, bytes: &[u8]) -> Result<E, PersistenceError>;
}
