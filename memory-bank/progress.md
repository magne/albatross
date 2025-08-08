# Progress

* **Current Status:** Phase 1, Step 7 completed. Ready to start Step 8 (Role-Based Authorization). Plan file `doc/plans/phase-1-plan.md` (v7) synchronized.
* **Completed Features/Milestones:**
  * **Internal Cleanup:** Refactored `projection-worker` handlers to align ID types (VARCHAR) with DB schema and fixed related compilation errors/warnings. - **DONE**
  * **Phase 1, Step 7:** API Key Authentication & Management (Backend - `apps/api-gateway`, `libs/core-lib`) - **DONE**
    * Implemented API key generation, authentication (cache-based), and revocation.
    * Added `POST /api/users/{user_id}/apikeys` and `DELETE /api/users/{user_id}/apikeys/{key_id}` endpoints.
    * Implemented `GenerateApiKeyHandler` & `RevokeApiKeyHandler`.
    * Implemented `api_key_auth` middleware.
    * Added integration tests (`api_key_routes.rs`).
    * Refined logging.
  * **Phase 1, Step 6:** DB Logic in Projections & Notifications (Backend - `apps/projection-worker`) - **DONE**
    * Set up DB connection pool (`sqlx::PgPool`).
    * Ran migrations on startup.
    * Updated projection handlers for DB writes (`sqlx`).
    * Implemented Redis Pub/Sub notification publishing.
    * Added `UserApiKey` read model and migration (`02__add_user_api_keys.sql`).
  * **Phase 1, Step 5:** Real Infrastructure Adapters & Integration Tests (Backend - `libs/core-lib`) - **DONE**
    * Added dependencies (`sqlx`, `lapin`, `redis-rs`, `testcontainers-rs`).
    * Implemented `PostgresEventRepository`, `RabbitMqEventBus`, `RedisCache`, `RedisEventBus`.
    * Added integration tests using `testcontainers-rs`.
  * **Phase 1, Step 4:** Projection Worker Skeleton & Migrations (Backend - `apps/projection-worker`) - **DONE**
    * Created `apps/projection-worker` service.
    * Defined initial Read Model schemas (`tenants`, `users`) and migration (`01__initial_read_models.sql`).
    * Embedded migrations (`refinery`).
    * Implemented basic event consumption loop (in-memory).
  * **Phase 1, Step 3:** Initial Command Handlers & API Gateway Setup (Backend - `apps/api-gateway`) - **DONE**
    * Implemented command handlers (`RegisterUserHandler`, `CreateTenantHandler`, `ChangePasswordHandler`, `GenerateApiKeyHandler` - initial version).
    * Implemented basic command dispatch, Axum routes, state injection, error mapping.
  * **Phase 1, Step 2:** Initial Domain Model & Protobuf (Backend - `libs/proto`, `libs/core-lib`) - **DONE**
    * Defined Protobuf messages (`Tenant`, `User`, `PIREP`).
    * Implemented Aggregate roots (`Tenant`, `User`, `Pirep`).
  * **Phase 1, Step 1:** Core ES/CQRS Libs & In-Memory Adapters (Backend - `libs/core-lib`) - **DONE**
    * Defined core Ports (Traits).
    * Implemented `InMemoryEventRepository`, `InMemoryEventBus`, `InMemoryCache`.
    * Added dependencies, organized modules.
  * **Phase 0:** Project Setup, Scaffolding, Initial Config - **DONE**
    * Finalized core tech stack.
    * Established monorepo structure.
    * Created initial service/library skeletons.
    * Configured Protobuf build process.
    * Scaffolded frontend project.
    * Integrated Biome.
    * Created basic Docker Compose infra definition.
    * Created placeholder Helm definitions.
    * Set up basic GitHub Actions CI.
    * Implemented basic frontend asset embedding.

* **Work In Progress:** None. Ready for Phase 1, Step 8.

* **Upcoming Work (Phase 1, Step 8):** Role-Based Authorization (Backend - `apps/api-gateway`, `libs/core-lib`, `apps/projection-worker`) - **NEXT**
  * Define `Role` enum (`PlatformAdmin`, `TenantAdmin`, `Pilot`).
  * Enhance `User` aggregate (state, events, commands) to manage roles.
  * Update Protobuf definitions (`Role` enum, commands, events, `AuthenticatedUser`).
  * Update `projection-worker`: Add migration (`03__add_user_roles.sql`) for `roles JSONB` column in `users` table; update projection handlers.
  * Ensure `AuthenticatedUser` in cache includes roles.
  * Implement authorization logic (Axum middleware `RequireRoleLayer` and/or handler checks).
  * Apply authorization to API routes.
  * Add unit and integration tests.
  * (Subsequent Steps): API Query Endpoints, WebSockets, Frontend UI, PIREP Flow, Admin Setup, Docker Deployment, Testing, Docs.

* **Known Issues/Bugs:** None specific yet.
  * *Potential Risks:* Inherent complexity of ES/CQRS and microservices. Managing schema evolution. Ensuring robust multi-tenancy isolation. Operational overhead of chosen stack. Password handling in aggregates needs careful review. `LoginUser` command/handler flow needs implementation/refinement.

* **Decision Log:** (Summary - See `activeContext.md` for more detail)
  * Project Name: Albatross.
  * Architecture: ES/CQRS, Hexagonal, Microservices (planned), Multi-tenant.
  * Backend Stack: Rust/Axum, PostgreSQL, RabbitMQ, Redis.
  * Frontend Stack: React/Vite/SWC, Tailwind CSS v4, Headless UI.
  * Infrastructure Stack: PostgreSQL, RabbitMQ, Redis.
  * Repo Structure: Monorepo (Cargo Workspace, PNPM).
  * Serialization: Protobuf (`prost`, binary `bytea`).
  * Deployment: 3 models (Single Executable, Docker Compose, K8s).
  * Licensing: Dual AGPLv3+Commercial or BSL preferred.
  * Linting/Formatting: Biome (JS/TS/JSON), cargo fmt/clippy (Rust).
  * Migrations: `refinery` + `sqlx-cli`.
  * API Key Auth: Cache-based lookup.
