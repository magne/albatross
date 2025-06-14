# Progress

* **Current Status:** Phase 1, Step 7 completed. Ready to start Step 8 (Role-Based Authorization).
* **Completed Features/Milestones:**
  * **Phase 1, Step 7:**
    * Implemented API key generation, authentication (cache-based), and revocation in `api-gateway`.
    * Added `POST /api/users/{user_id}/apikeys` and `DELETE /api/users/{user_id}/apikeys/{key_id}` endpoints.
    * Implemented `GenerateApiKeyHandler` (returns plain key once, stores hash in aggregate, populates cache).
    * Implemented `RevokeApiKeyHandler` (stores event in aggregate, invalidates cache entries).
    * Implemented `api_key_auth` middleware using direct cache lookup.
    * Added integration tests (`api_key_routes.rs`) for the full lifecycle.
    * Refined logging to avoid exposing sensitive key material.
  * **Phase 1, Step 6:**
    * Successfully implemented Projection Worker with RabbitMQ Consumer, PostgreSQL writes, and Redis notifications.
    * Fixed `sqlx` offline query compilation issues.
    * Verified all tests pass successfully.
  * **Phase 1, Step 5:**
    * Implemented real infrastructure adapters (`PostgresEventRepository`, `RabbitMqEventBus`, `RedisCache`, `RedisEventBus`) in `libs/core-lib`.
    * Added corresponding dependencies (`sqlx`, `lapin`, `redis-rs`, `testcontainers-rs`, `testcontainers-modules`) to `libs/core-lib`.
    * Implemented basic integration tests for adapters using `testcontainers-rs`.
    * Refactored `Repository` trait and `InMemoryEventRepository` to handle raw event data.
    * Removed flawed default `Aggregate::load_from_data` method.
    * Verified `libs/core-lib` compiles successfully (with expected warnings).
  * **Phase 1, Step 4:**
    * Created `apps/projection-worker` service skeleton and added to workspace.
    * Defined initial read model schemas (`tenants`, `users`) and migration file.
    * Embedded migrations in `projection-worker` using `refinery`.
    * Implemented basic event consumption loop in `projection-worker` using `InMemoryEventBus`.
    * Implemented placeholder projection handlers (`handle_tenant_created`, `handle_user_registered`).
    * Verified `apps/projection-worker` compiles successfully (with expected warnings).
  * **Phase 1, Step 3:**
    * Implemented command handlers (`RegisterUserHandler`, `CreateTenantHandler`, `ChangePasswordHandler`, `GenerateApiKeyHandler`) in `apps/api-gateway`.
    * Implemented basic command dispatch logic: DTOs, Axum route handlers (`/api/users`, `/api/tenants`), state injection, error mapping to HTTP responses in `api-gateway`.
    * Added necessary dependencies and error handling infrastructure to `api-gateway`.
    * Added `From<AggregateError>` implementations to `CoreError` in `libs/core-lib`.
    * Verified `apps/api-gateway` compiles successfully (with expected warnings for unused code).
  * **Phase 1, Steps 1 & 2:**
    * Defined core ES/CQRS Ports (traits) in `libs/core-lib`.
    * Implemented in-memory adapters (`InMemoryEventRepository`, `InMemoryEventBus`, `InMemoryCache`) in `libs/core-lib`.
    * Defined Protobuf messages for `Tenant`, `User`, and `PIREP` commands/events in `libs/proto`.
    * Implemented initial Aggregate roots (`Tenant`, `User`, `Pirep`) in `libs/core-lib`.
    * Verified `libs/core-lib` and `libs/proto` compile successfully.
  * **Phase 0:**
    * Finalized core technology stack (Axum, React, Vite/SWC, Tailwind v4, Postgres, RabbitMQ, Redis, Protobuf).
    * Established monorepo structure (`apps/`, `libs/`) with Git, Cargo workspace, PNPM.
    * Created initial Rust service/library skeletons (`api-gateway`, `core-lib`, `proto`).
    * Configured basic Protobuf build process (`libs/proto/build.rs`).
    * Scaffolded frontend project (`apps/web-ui`) using React, Vite, SWC, React Router, Tailwind v4.
    * Integrated Biome for JS/TS linting/formatting.
    * Created basic Docker Compose infrastructure definition (`docker-compose.infra.yml`).
    * Created placeholder for Helm infrastructure definitions (`infra/helm/README.md`).
    * Set up basic GitHub Actions CI workflow (`.github/workflows/ci.yml`).
    * Implemented basic frontend asset embedding in `api-gateway` using `rust-embed`.

* **Work In Progress:** None. Ready for Phase 1, Step 8.

* **Upcoming Work (Phase 1, Step 8):**
  * Define basic roles (e.g., PlatformAdmin, TenantAdmin, Pilot).
  * Enhance aggregates (User, Tenant) to store/manage roles.
  * Update projections and read models to include role information.
  * Implement authorization logic (e.g., middleware or checks within handlers) in `api-gateway` based on roles extracted from authenticated user context (initially from API key, later JWT).
  * Add tests for role-based access control.
  * (Subsequent Steps): Containerization, infrastructure setup, testing.

* **Known Issues/Bugs:** None specific yet.
  * *Potential Risks:* Inherent complexity of ES/CQRS and microservices. Managing schema evolution. Ensuring robust multi-tenancy isolation. Operational overhead of chosen stack. Password handling in aggregates needs careful review. `LoginUser` command/handler flow needs implementation/refinement.

* **Decision Log:** (Summary of key decisions from initial planning & recent updates)
  * Project Name: Albatross (Finalized for now).
  * Architecture: ES/CQRS, Hexagonal (Ports & Adapters), Microservices (planned), Multi-tenant.
  * Backend Stack: Rust / Axum framework, PostgreSQL, RabbitMQ, Redis.
  * Frontend Stack: React, React Router, Vite (with SWC), Tailwind CSS v4, Headless UI. (Vue & Svelte to be implemented in Phase 1.5 for comparison).
  * Infrastructure Stack ("Scenario B"): PostgreSQL (Events/Reads), RabbitMQ (Event Bus), Redis (Cache/PubSub).
  * Repo Structure: Monorepo (Cargo Workspace, PNPM).
  * Serialization: Protobuf (using `prost`, stored as binary `bytea`).
  * Deployment: Support 3 models (Single Executable uses In-Memory Adapters/SQLite, Docker Compose uses real infra, K8s uses real infra). Phase 1 includes basic Docker Compose support.
  * Licensing: Dual AGPLv3+Commercial or BSL model preferred over standard OSI licenses due to commercial restrictions requirement.
  * Infrastructure Management: Separate reusable definitions (Docker Compose files, Helm Charts) from application code.
  * Linting/Formatting: Biome (JS/TS/JSON), cargo fmt/clippy (Rust).
  * Initial Setup: Platform Admin created on first run with logged one-time password.
  * UI Components: Headless UI chosen for React.
  * Real-time: WebSockets included in Phase 1 MVP.
  * Migrations: `refinery` crate chosen, but using `sqlx-cli` for offline query preparation.
  * API Key Auth: Cache-based lookup using plain text key. Revocation invalidates cache.
