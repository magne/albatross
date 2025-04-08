// Declare modules within the adapters directory
pub mod in_memory_cache;
pub mod in_memory_event_bus;
pub mod in_memory_repository;
pub mod postgres_repository;
pub mod rabbitmq_event_bus;
pub mod redis_cache;
pub mod redis_event_bus;

// TODO: Add feature flags (e.g., "postgres", "rabbitmq", "redis", "in_memory_infra")
//       to conditionally compile these adapters and allow selection at runtime
//       in application services (api-gateway, projection-worker).

// Optional: Re-export key adapter types for easier access from crate root
// pub use redis_event_bus::RedisEventBus;
// pub use redis_cache::RedisCache;
// pub use rabbitmq_event_bus::RabbitMqEventBus;
// pub use postgres_repository::PostgresEventRepository;
// pub use in_memory_cache::InMemoryCache;
// pub use in_memory_event_bus::InMemoryEventBus;
// pub use in_memory_repository::InMemoryEventRepository;
