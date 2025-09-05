# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **CRITICAL: Missing Events Table** - Fixed CQRS/ES system failure due to missing events table migration
  - Moved all database migrations from projection-worker to api-gateway (command side ownership)
  - Created events table migration (03__create_events_table.sql) with proper schema, indexes, and multi-tenancy support
  - Updated api-gateway to run migrations on startup
  - Updated projection-worker to verify tables exist instead of running migrations
  - Established correct CQRS architecture: Command side creates events, Query side reads them
  - Added proper startup order validation and error handling

### Changed
- **Architecture**: Corrected CQRS/ES database ownership - api-gateway (command side) now owns and creates database schema
- **Migration Strategy**: Centralized database migrations in api-gateway to ensure events table exists before projection-worker starts
- **Startup Order**: Defined clear dependency - api-gateway must start before projection-worker

### Added
- Events table with multi-tenancy support (`tenant_id` column)
- Migration verification in projection-worker startup
- Proper error messages for missing database tables
- Database schema ownership documentation

## [0.1.0] - 2025-01-XX

### Added
- Initial CQRS/ES system with Event Sourcing
- PostgreSQL event store and read models
- RabbitMQ event bus for inter-service communication
- Redis caching and Pub/Sub for real-time updates
- WebSocket real-time delivery system
- Multi-tenant architecture
- API key authentication system
- User and tenant management
- React frontend with Tailwind CSS
- Docker Compose infrastructure setup

### Technical Decisions
- Rust/Axum backend with CQRS/ES architecture
- PostgreSQL for event store and read models
- RabbitMQ for event distribution
- Redis for caching and real-time notifications
- React/Vite frontend
- Protobuf for event/command serialization
