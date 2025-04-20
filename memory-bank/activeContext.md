# Active Context

* **Current Focus:** Phase 1, Step 7 - Add Basic Authentication (API Key).
* **Recent Changes (Phase 1, Step 6 Completion):**
  * Successfully implemented Projection Worker with RabbitMQ Consumer, PostgreSQL writes, and Redis notifications.
  * Fixed `sqlx` offline query compilation issues:
    * Installed `sqlx-cli` for database management
    * Renamed migration file from `V1__initial_read_models.sql` to `01__initial_read_models.sql`
    * Ran migrations with `cargo sqlx migrate run`
    * Added type hints (`::Uuid`) to SQL queries
    * Generated offline query data with `cargo sqlx prepare --workspace`
  * All tests now pass successfully.
* **Recent Changes (Phase 1, Step 5 Completion):**
  * Implemented real infrastructure adapters (`PostgresEventRepository`, `RabbitMqEventBus`, `RedisCache`, `RedisEventBus`) in `libs/core-lib`.
  * Added corresponding dependencies (`sqlx`, `lapin`, `redis-rs`, `testcontainers-rs`, `testcontainers-modules`) to `libs/core-lib`.
  * Implemented basic integration tests for adapters using `testcontainers-rs`.
  * Refactored `Repository` trait to handle raw event data (`StoredEventData`).
  * Refactored `InMemoryEventRepository` to match the updated trait.
  * Removed flawed default `Aggregate::load_from_data` method (loading logic moved to consumers).
  * Verified `libs/core-lib` compiles successfully (with expected warnings).
* **Next Steps (Phase 1, Step 7 Start):**
  * Enhance User aggregate with API key revocation support.
  * Implement API key authentication middleware in `api-gateway`.
  * Add API key management endpoints to `api-gateway`.
  * Update projection worker to handle API key events.
  * Update database schema for API key storage.
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
  * Migrations: Using both `refinery` (runtime) and `sqlx-cli` (offline preparation).
* **Key Patterns/Preferences:**
  * Prioritize Open Source components and minimal vendor lock-in.
  * Aim for good Developer Experience (DX), including debugging support for microservices potentially running outside k3s.
  * Maintain clear separation between application logic and reusable infrastructure definitions.
  * **Workflow:** Stop after completing each step in the current plan (`doc/plans/phase-1-plan.md`). Update Memory Bank (`activeContext.md`, `progress.md`) after each step completion. Ensure plan formatting uses consistent spacing (like `phase-1-plan.md`).
* **Learnings/Insights:**
  * Using both `refinery` and `sqlx-cli` provides good balance: runtime migrations with `refinery`, offline query validation with `sqlx-cli`.
  * Type hints (e.g., `::Uuid`) in SQL queries help `sqlx` macro understand types during offline preparation.
  * Migration file naming requirements differ between tools (`V1__` for `refinery`, `01__` for `sqlx-cli`).
  * Analyzed trade-offs for backend/frontend frameworks, component libraries, event stores, multi-tenancy strategies, deployment costs, licensing, and repo structures.
  * Established the feasibility of the 3 deployment models with careful abstraction.
  * Recognized the complexity introduced by microservices, especially for Model 1 deployment.
  * `replace_in_file` tool seems unreliable for larger markdown file edits; `write_to_file` used as fallback.
  * Error reporting feedback loop can sometimes be stale; `cargo check` needed for confirmation.
  * Implementing `From<SpecificError>` for `GeneralError` is key for using `?` effectively across layers.
  * Axum state management with `Arc` provides straightforward dependency injection.
  * Testcontainers setup requires careful attention to dependency versions and import paths (`testcontainers` vs `testcontainers-modules`).
  * `redis-rs` async PubSub API requires specific handling (`PubSubConnection`, `into_on_message`).
