# Active Context

* **Current Focus:** Phase 1, Step 5 - Implement First Query Endpoints & Caching.
* **Recent Changes (Phase 1, Step 4 Completion):**
  * Created `apps/projection-worker` service skeleton and added to workspace.
  * Defined initial read model schemas (`tenants`, `users`) and migration file (`V1__initial_read_models.sql`).
  * Embedded migrations in `projection-worker` using `refinery`.
  * Implemented basic event consumption loop in `projection-worker` using `InMemoryEventBus` (for now).
  * Implemented placeholder projection handlers (`handle_tenant_created`, `handle_user_registered`) with TODOs for DB updates and Redis notifications.
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
* **Next Steps (Phase 1, Step 5 Start):**
  * Develop API query endpoints in `apps/api-gateway` (e.g., `GET /api/tenants`, `GET /api/users`).
  * Implement basic query handlers/logic to read directly from read models (requires DB connection setup - deferring actual DB interaction).
  * Implement basic Redis caching for these query endpoints using the `Cache` port/adapter.
* **Active Decisions:**
  * Project Name: Albatross (Finalized for now).
  * Architecture: ES/CQRS, Hexagonal (Ports & Adapters), Microservices (planned), Multi-tenant.
  * Backend Stack: Axum (Rust), Postgres, RabbitMQ, Redis.
  * Frontend Stack: React, React Router, Vite (with SWC), Tailwind CSS v4, Headless UI.
  * Structure: Monorepo (Cargo Workspace, PNPM).
  * Deployment: 3 Models defined (Single Executable uses In-Memory Adapters).
  * Serialization: Protobuf (stored as binary `bytea`).
  * Linting/Formatting: Biome (JS/TS/JSON), cargo fmt/clippy (Rust).
  * Initial Setup: Platform Admin created on first run with logged one-time password.
  * UI Components: Headless UI chosen.
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
