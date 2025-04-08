# Active Context

* **Current Focus:** Phase 1, Step 6 - Implement DB Logic in Projections & Notifications.
* **Recent Changes (Phase 1, Step 5 Completion):**
  * Implemented real infrastructure adapters (`PostgresEventRepository`, `RabbitMqEventBus`, `RedisCache`, `RedisEventBus`) in `libs/core-lib`.
  * Added corresponding dependencies (`sqlx`, `lapin`, `redis-rs`, `testcontainers-rs`, `testcontainers-modules`) to `libs/core-lib`.
  * Implemented basic integration tests for adapters using `testcontainers-rs`.
  * Refactored `Repository` trait to handle raw event data (`StoredEventData`).
  * Refactored `InMemoryEventRepository` to match the updated trait.
  * Removed flawed default `Aggregate::load_from_data` method (loading logic moved to consumers).
  * Verified `libs/core-lib` compiles successfully (with expected warnings).
* **Recent Changes (Phase 1, Step 4 Completion):**
  * Created `apps/projection-worker` service skeleton and added to workspace.
  * Defined initial read model schemas (`tenants`, `users`) and migration file (`V1__initial_read_models.sql`).
  * Embedded migrations in `projection-worker` using `refinery`.
  * Implemented basic event consumption loop in `projection-worker` using `InMemoryEventBus`.
  * Implemented placeholder projection handlers (`handle_tenant_created`, `handle_user_registered`).
  * Verified `apps/projection-worker` compiles successfully (with expected warnings).
* **Recent Changes (Phase 1, Step 3 Completion):**
  * Implemented command handlers (`RegisterUserHandler`, `CreateTenantHandler`, `ChangePasswordHandler`, `GenerateApiKeyHandler`) in `apps/api-gateway`.
  * Implemented basic command dispatch logic: DTOs, Axum route handlers (`/api/users`, `/api/tenants`), state injection, error mapping to HTTP responses in `api-gateway`.
  * Added necessary dependencies and error handling infrastructure to `api-gateway`.
  * Added `From<AggregateError>` implementations to `CoreError` in `libs/core-lib`.
  * Verified `apps/api-gateway` compiles successfully (with expected warnings for unused code).
* **Recent Changes (Phase 1, Steps 1 & 2 Completion):**
  * Defined core ES/CQRS Ports (traits) in `libs/core-lib`.
  * Implemented in-memory adapters (`InMemoryEventRepository`, `InMemoryEventBus`, `InMemoryCache`) in `libs/core-lib`.
  * Defined Protobuf messages for `Tenant`, `User`, and `PIREP` commands/events in `libs/proto`.
  * Implemented initial Aggregate roots (`Tenant`, `User`, `Pirep`) in `libs/core-lib`.
  * Verified `libs/core-lib` and `libs/proto` compile successfully.
  * (Phase 0): Finalized core technology stack, set up monorepo, created skeletons, configured Protobuf build, set up frontend, created infra placeholders, basic CI, basic embedding.
* **Next Steps (Phase 1, Step 6 Start):**
  * Set up database connection pool (`sqlx::PgPool`) in `projection-worker`.
  * Run database migrations on startup in `projection-worker`.
  * Update projection handlers (`handle_tenant_created`, `handle_user_registered`) to perform actual DB INSERT/UPDATE operations using `sqlx`.
  * Implement Redis Pub/Sub notification publishing from projection handlers using the `RedisEventBus` adapter.
* **Future Steps (Phase 1.5):**
  * A plan for Phase 1.5 (MVP Refinement & Foundation Hardening) has been created at `doc/plans/phase-1.5-plan.md`. This phase includes robust Auth/Authz, SQLite support, Docker/Helm setup, basic Observability, and Vue/Svelte MVP implementations.
* **Active Decisions:**
  * Project Name: Albatross (Finalized for now).
  * Architecture: ES/CQRS, Hexagonal (Ports & Adapters), Microservices (planned), Multi-tenant.
  * Backend Stack: Axum (Rust), Postgres, RabbitMQ, Redis.
  * Frontend Stack: React, React Router, Vite (with SWC), Tailwind CSS v4, Headless UI. (Vue & Svelte to be implemented in Phase 1.5 for comparison).
  * Structure: Monorepo (Cargo Workspace, PNPM).
  * Deployment: 3 Models defined (Single Executable uses In-Memory Adapters/SQLite, Docker Compose uses real infra, K8s uses real infra). Phase 1 includes basic Docker Compose support.
  * Serialization: Protobuf (stored as binary `bytea`).
  * Linting/Formatting: Biome (JS/TS/JSON), cargo fmt/clippy (Rust).
  * Initial Setup: Platform Admin created on first run with logged one-time password.
  * UI Components: Headless UI chosen for React.
  * Real-time: WebSockets included in Phase 1 MVP.
  * Migrations: `refinery` crate chosen.
* **Key Patterns/Preferences:**
  * Prioritize Open Source components and minimal vendor lock-in.
  * Aim for good Developer Experience (DX), including debugging support for microservices potentially running outside k3s.
  * Maintain clear separation between application logic and reusable infrastructure definitions.
  * **Workflow:** Stop after completing each step in the current plan (`doc/plans/phase-1-plan.md`). Update Memory Bank (`activeContext.md`, `progress.md`) after each step completion. Ensure plan formatting uses consistent spacing (like `phase-1-plan.md`).
* **Learnings/Insights:**
  * Analyzed trade-offs for backend/frontend frameworks, component libraries, event stores, multi-tenancy strategies, deployment costs, licensing, and repo structures.
  * Established the feasibility of the 3 deployment models with careful abstraction.
  * Recognized the complexity introduced by microservices, especially for Model 1 deployment.
  * `replace_in_file` tool seems unreliable for larger markdown file edits; `write_to_file` used as fallback.
  * Error reporting feedback loop can sometimes be stale; `cargo check` needed for confirmation.
  * Implementing `From<SpecificError>` for `GeneralError` is key for using `?` effectively across layers.
  * Axum state management with `Arc` provides straightforward dependency injection.
  * Testcontainers setup requires careful attention to dependency versions and import paths (`testcontainers` vs `testcontainers-modules`).
  * `redis-rs` async PubSub API requires specific handling (`PubSubConnection`, `into_on_message`).
