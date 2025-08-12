# Phase 1 Plan: Core MVP & ES/CQRS Foundation (v7)

This plan outlines the steps for implementing the Minimum Viable Product (MVP) features for the Albatross Virtual Airline Management Platform, focusing on establishing the core Event Sourcing (ES) / Command Query Responsibility Segregation (CQRS) loop, Hexagonal Architecture, basic authentication, real-time UI updates via WebSockets, and basic Docker deployment capabilities. It includes provisions for in-memory adapters for testing and single-executable deployment, and real adapters tested with testcontainers.

**MVP Features:**

* Platform Admin initial setup.
* Tenant (Airline) Creation (by Platform Admins).
* User Registration (PlatformAdmin, TenantAdmin, Pilot roles).
* User Login & Password Management.
* API Key Generation/Management.
* Basic PIREP Submission & Viewing.
* Real-time UI updates for relevant changes.
* **Runnable via Docker Compose.**

**Technology Stack Highlights:**

* Backend: Rust, Axum, PostgreSQL (Event Store & Read Models), RabbitMQ (Event Bus), Redis (Cache & Pub/Sub), `prost` (Protobuf), `refinery` (Migrations), `sqlx` (Postgres Driver - preferred), `lapin` (RabbitMQ Driver), `redis-rs` (Redis Driver), `testcontainers-rs`.
* Frontend: TypeScript, React, Vite (SWC), Tailwind CSS v4, Headless UI, `react-use-websocket`.
* Architecture: ES/CQRS, Hexagonal (Ports & Adapters), Microservices (planned, starting with `api-gateway` & `projection-worker`), Multi-tenant.

**Plan Steps:**

1. **Core ES/CQRS Libs & In-Memory Adapters (Backend - `libs/core-lib`) - DONE**
    * Define core **Ports** (Rust traits).
    * Implement **In-Memory Adapters** (`InMemoryEventRepository`, `InMemoryEventBus`, `InMemoryCache`).
    * Add dependencies, organize modules.
    * Verify `libs/core-lib` compiles.

2. **Define Initial Domain Model & Protobuf (Backend - `libs/proto`, `libs/core-lib`) - DONE**
    * Define Protobuf messages (`Tenant`, `User`, `PIREP`).
    * Configure `libs/proto` build script.
    * Implement Aggregate roots (`Tenant`, `User`, `Pirep`).
    * Verify `libs/core-lib` compiles.

3. **Implement Initial Command Handlers & API Gateway Setup (Backend - `apps/api-gateway`) - DONE**
    * Implement command handlers (`RegisterUserHandler`, `CreateTenantHandler`, `ChangePasswordHandler`, `GenerateApiKeyHandler` - initial version).
    * Implement basic command dispatch, Axum routes, state injection, error mapping.
    * Verify `apps/api-gateway` compiles.

4. **Implement Initial Projections (Worker Setup) & Migrations (Backend - `apps/projection-worker`) - DONE**
    * Create `apps/projection-worker` service.
    * Define initial Read Model schemas (`tenants`, `users`) and migration (`01__initial_read_models.sql`).
    * Embed migrations (`refinery`).
    * Implement basic event consumption loop (in-memory).
    * Implement placeholder projection handlers.
    * Verify `apps/projection-worker` compiles.

5. **Implement Real Infrastructure Adapters & Integration Tests (Backend - `libs/core-lib`) - DONE**
    * Add dependencies (`sqlx`, `lapin`, `redis-rs`, `testcontainers-rs`).
    * Implement `PostgresEventRepository`, `RabbitMqEventBus`, `RedisCache`, `RedisEventBus`.
    * Add integration tests using `testcontainers-rs`.
    * Ensure adapter selection is configurable.

6. **Implement DB Logic in Projections & Notifications (Backend - `apps/projection-worker`) - DONE**
    * Set up DB connection pool (`sqlx::PgPool`).
    * Run migrations on startup.
    * Update projection handlers for DB writes (`sqlx`).
    * Implement Redis Pub/Sub notification publishing.
    * Added `UserApiKey` read model and migration (`02__add_user_api_keys.sql`).

7. **API Key Authentication & Management (Backend - `apps/api-gateway`, `libs/core-lib`) - DONE**
    * Implemented API key generation, authentication (cache-based), and revocation.
    * Added `POST /api/users/{user_id}/apikeys` and `DELETE /api/users/{user_id}/apikeys/{key_id}` endpoints.
    * Implemented `GenerateApiKeyHandler` & `RevokeApiKeyHandler`.
    * Implemented `api_key_auth` middleware.
    * Added integration tests (`api_key_routes.rs`).
    * Refined logging.

8. **Role-Based Authorization (Backend - `apps/api-gateway`, `libs/core-lib`, `apps/projection-worker`) - DONE**
    * Added `application/authz.rs` with `AuthRole`, `Requirement`, `authorize()`, `parse_role()`.
    * Enforced PlatformAdmin-only tenant creation and RBAC constraints in user registration and API key endpoints.
    * Implemented bootstrap registration path (first PlatformAdmin w/out auth).
    * Added self/tenant admin or platform admin logic for API key lifecycle.
    * Middleware enriches legacy cache entries (adds role) via dynamic event replay.
    * User aggregate invariants: PlatformAdmin must have no tenant; others must have tenant.
    * Read model already contains `role` field (no new migration).
    * Pending: negative RBAC integration tests (forbidden scenarios), unit tests for `authorize()`.

9. **Implement API Query Endpoints & Caching (Backend - `apps/api-gateway`) - IN PROGRESS**
    * Provide `GET /api/tenants` and `GET /api/users` endpoints.
    * RBAC-scoped results.
    * Redis cache-aside for result sets with short TTL.
    * Add missing RBAC negative tests (from Step 8).
    * Prepare for WebSocket invalidation (Step 10).

10. **Implement WebSocket Logic (Backend - `apps/api-gateway`)**
    * Configure Axum WebSocket endpoint (`/api/ws`).
    * Implement connection handling, authentication, and subscription logic using the `RedisEventBus` subscriber functionality.
    * Forward messages from Redis Pub/Sub to connected WebSocket clients.

11. **Implement Initial Frontend UI & Real-time Updates (Frontend - `apps/web-ui`)**
    * Integrate Headless UI with Tailwind CSS v4.
    * Build React components for MVP features (Login, API Key, Change Pwd, Tenant Create/List, User Reg/List).
    * Implement WebSocket client logic (`react-use-websocket`) to connect and handle real-time updates.
    * Connect components to REST backend endpoints.

12. **Basic PIREP Flow (Backend & Frontend)**
    * Implement `SubmitPIREP` command handler in `api-gateway`.
    * Implement `PIREPSubmitted` projection handler in `projection-worker` (including DB update & Redis notification).
    * Develop PIREP REST API endpoints (`POST/GET /api/pireps`) in `api-gateway`.
    * Create PIREP UI form/view in `web-ui` with real-time updates.

13. **Initial Platform Admin Setup (Backend - `apps/api-gateway`)**
    * Implement startup logic to create initial `PlatformAdmin` user if none exists, logging one-time password.

14. **Docker Deployment Setup**
    * Create `Dockerfile` for `apps/api-gateway`.
    * Create `Dockerfile` for `apps/projection-worker`.
    * Create `docker-compose.application.yml` (or similar) to define services for `api-gateway` and `projection-worker`, linking them to the infrastructure services defined in `docker-compose.infra.yml`.
    * Ensure application can be launched using `docker compose -f docker-compose.infra.yml -f docker-compose.application.yml up`.

15. **Testing & Refinement**
    * Write/update unit tests (using In-Memory adapters).
    * Write/update integration tests (using `testcontainers-rs` adapters).
    * Write/update E2E tests (Playwright) for core UI flows.
    * Refine implementation based on tests.

16. **Documentation**
    * Update this plan file (`doc/plans/phase-1-plan.md`).
    * Continuously update Memory Bank files (`activeContext.md`, `progress.md`, `systemPatterns.md`).
    * Add necessary code comments and documentation.

**Cross-Cutting Improvements / Technical Debt Identified (Rolling List):**
* Replace placeholder password hashing in registration with Argon2 (consistency with API key hashing).
* Extract shared aggregate replay helper (reduce duplication in handlers & middleware).
* Centralize TTL constants for cache usage (auth vs queries).
* Add negative RBAC integration tests & unit tests for `authorize()`.
* Introduce unified HTTP error mapping helper.
* Add structured tracing spans for new query endpoints.
* Consider early pagination abstraction for queries (limit enforcement).
* Prepare cache invalidation hook for Step 10 (WebSocket notifications).
