# Active Context

* **Current Focus:** Phase 1, Step 8 (Role-Based Authorization) implementation completed (core RBAC enforcement in place). Preparing to begin Phase 1, Step 9 (Query Endpoints & Caching). Plan file `doc/plans/phase-1-plan.md` (v7) synchronized; step-specific detailed plan in `doc/plans/phase-1-step-8-plan.md` executed (tests for negative RBAC paths still to be added).
* **Recent Changes (Internal Cleanup):**
  * Refactored `projection-worker` handlers to align ID types (VARCHAR) with DB schema and fixed related compilation errors/warnings.
* **Recent Changes (Phase 1, Step 7 Completion):**
  * Implemented API key generation, authentication, and revocation in `api-gateway`.
  * Added `POST /api/users/{user_id}/apikeys` and `DELETE /api/users/{user_id}/apikeys/{key_id}` endpoints.
  * Implemented `GenerateApiKeyHandler` which returns the plain text key once and stores hash in `User` aggregate (`ApiKeyGenerated` event).
  * Implemented `RevokeApiKeyHandler` which triggers `ApiKeyRevoked` event in `User` aggregate.
  * Implemented `api_key_auth` middleware using cache-based lookup (plain text key as cache key).
  * Added cache storage for `AuthenticatedUser` (keyed by plain text key) and `key_id -> plain_key` mapping (for revocation).
  * Implemented cache invalidation logic in `RevokeApiKeyHandler` to remove both entries upon successful revocation.
  * Added integration tests (`api_key_routes.rs`) covering the generate-authenticate-revoke lifecycle.
  * Refined logging in handlers and middleware to avoid logging sensitive key material.
  * Cleaned up unused imports and addressed clippy warnings.
* **Recent Changes (Phase 1, Step 6 Completion):**
  * Successfully implemented Projection Worker with RabbitMQ Consumer, PostgreSQL writes, and Redis notifications.
  * Fixed `sqlx` offline query compilation issues.
  * All tests passed successfully.
* **Recent Changes (Phase 1, Step 8 Completion):**
  * Added RBAC policy module (`application/authz.rs`) with `AuthRole`, `Requirement`, `authorize()`, `parse_role()`.
  * Protected `POST /api/tenants` route with API key auth middleware and PlatformAdmin-only check.
  * Implemented RBAC logic in `handle_register_user_request`:
    * Bootstrap path: unauthenticated creation allowed only for initial PlatformAdmin (no tenant).
    * Authenticated path: PlatformAdmin can create any user (rules on tenant_id presence); TenantAdmin restricted to own tenant and non-Platform roles; Pilot forbidden.
  * Implemented RBAC in API key generation & revocation handlers using `Requirement::SelfOrTenantAdmin` plus bootstrap allowance for first key.
  * Wrapped revoke and tenant routes in middleware layers; maintained legacy cache enrichment (adds role if absent).
  * Verified `users` read model already stores `role` (string) — no migration required; initial migration `01__initial_read_models.sql` sufficient.
  * All existing integration tests pass (added logic without breaking Step 7 tests). Negative/forbidden scenario tests still pending.
  * Confirmed user aggregate exposes `role()`, `tenant_id()`, `api_key_count()`.
* **Next Steps (Phase 1, Step 9 Start):**
  * Implement query endpoints: `GET /api/tenants`, `GET /api/users` (role-scoped: PlatformAdmin = all, TenantAdmin = own tenant, Pilot = perhaps limited/self).
  * Add integration tests for RBAC forbidden cases (TenantAdmin cross-tenant user creation, Pilot attempting privileged actions, non-bootstrap unauthorized registration, second unauthenticated API key attempt).
  * Add Redis-backed caching for query endpoints (key design: namespace + role/tenant scope hash).
  * Prepare message schema alignment for upcoming WebSocket broadcasting (Step 10).
  * Update documentation & memory after Step 9 completion.
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
  * API Key Auth: Cache-based lookup using plain text key. Revocation invalidates cache.
* **Key Patterns/Preferences:**
  * Prioritize Open Source components and minimal vendor lock-in.
  * Aim for good Developer Experience (DX), including debugging support for microservices potentially running outside k3s.
  * Maintain clear separation between application logic and reusable infrastructure definitions.
  * **Workflow:** Stop after completing each step in the current plan (`doc/plans/phase-1-plan.md`). Update Memory Bank (`activeContext.md`, `progress.md`) after each step completion. Ensure plan formatting uses consistent spacing.
  * Cache invalidation logic in command handlers should run *after* successful event persistence/publishing. Cache errors should be logged but generally not fail the command.
* **Learnings/Insights:**
  * Ensure consistency between code data types (e.g., IDs as String vs. Uuid) and database schema (VARCHAR vs. UUID).
  * Match arms in Rust must return compatible types; use `.map_err` and ensure all arms yield the same `Result` structure or use explicit `Ok(())` where appropriate.
  * Using both `refinery` and `sqlx-cli` provides good balance: runtime migrations with `refinery`, offline query validation with `sqlx-cli`.
  * Type hints (e.g., `::Uuid`) in SQL queries help `sqlx` macro understand types during offline preparation.
  * Migration file naming requirements differ between tools (`V1__` for `refinery`, `01__` for `sqlx-cli`).
  * `replace_in_file` tool seems unreliable for larger markdown file edits; `write_to_file` used as fallback. Tool can also corrupt files on save.
  * Error reporting feedback loop can sometimes be stale; `cargo check` needed for confirmation.
  * Implementing `From<SpecificError>` for `GeneralError` is key for using `?` effectively across layers.
  * Axum state management with `Arc` provides straightforward dependency injection.
  * Testcontainers setup requires careful attention to dependency versions and import paths (`testcontainers` vs `testcontainers-modules`).
  * `redis-rs` async PubSub API requires specific handling (`PubSubConnection`, `into_on_message`).
  * Cache-based API key auth requires careful handling of cache invalidation during revocation to prevent stale access. Storing a `key_id -> plain_key` mapping facilitates this.
  * Logging sensitive data like plain text API keys should be avoided, especially in error/warning paths.
