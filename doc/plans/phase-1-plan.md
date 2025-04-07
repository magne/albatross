# Phase 1 Plan: Core MVP & ES/CQRS Foundation (v6)

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

1. **Core ES/CQRS, Hexagonal Infra & WebSockets (Backend - `libs/core-lib`, `apps/api-gateway`, `apps/projection-worker`):**
    * Define core **Ports** (Rust traits) in `libs/core-lib` for `Aggregate`, `Command`, `Event`, `Repository`, `CommandHandler`, `EventHandler`, `EventPublisher`, `EventSubscriber`, `Cache`.
    * Implement **In-Memory Adapters** in `libs/core-lib`:
        * `InMemoryEventRepository`
        * `InMemoryEventBus`
        * `InMemoryCache`
    * Add necessary dependencies (`async-trait`, `thiserror`, `dashmap`, `tokio`, `moka`, etc.) to `libs/core-lib`.
    * Organize adapters and domain logic into modules.
    * Verify `libs/core-lib` compiles.

2. **Define Initial Domain Model (Backend - `libs/proto`, `libs/core-lib`):**
    * Define Protobuf messages in `libs/proto` for `Tenant`, `User`, `PIREP` Commands and Events.
    * Configure `libs/proto` build script and `lib.rs` to generate and include Rust code.
    * Verify `libs/proto` compiles.
    * Implement Aggregate roots (`Tenant`, `User`, `Pirep`) in `libs/core-lib/src/domain/`.
    * Verify `libs/core-lib` compiles with aggregates.

3. **Implement Initial Command/Event Flows (Backend - `apps/api-gateway`):**
    * Implement command handlers (`RegisterUserHandler`, `CreateTenantHandler`, `ChangePasswordHandler`, `GenerateApiKeyHandler`) using in-memory adapters initially.
    * Implement basic command dispatch logic (DTOs, Axum route handlers for `/api/users`, `/api/tenants`, state injection, basic error mapping).
    * Add necessary dependencies (`core-lib`, `axum`, `serde`, `uuid`, etc.) to `api-gateway`.
    * Implement `From<AggregateError>` for `CoreError` in `libs/core-lib`.
    * Verify `apps/api-gateway` compiles.

4. **Implement Initial Projections (Worker Setup) (Backend - `apps/projection-worker`):**
    * Create `apps/projection-worker` service skeleton (Cargo.toml, main.rs) and add to workspace.
    * Design initial Read Model schemas (`tenants`, `users`) and create SQL migration file (`V1__initial_read_models.sql`).
    * Embed migrations using `refinery`.
    * Implement basic event consumption loop using `InMemoryEventBus`.
    * Implement placeholder projection handlers (`handle_tenant_created`, `handle_user_registered`) with TODOs for DB updates/notifications.
    * Verify `apps/projection-worker` compiles.

5. **Implement Real Infrastructure Adapters & Integration Tests (Backend - `libs/core-lib`):** *(New Step 4b)*
    * Add dependencies: `sqlx` (with `runtime-tokio-rustls`, `postgres`, `uuid`, `chrono` features), `lapin`, `redis-rs`, `testcontainers-rs` (with relevant modules like `postgres`, `rabbitmq`, `redis`).
    * Implement `PostgresEventRepository` adapter using `sqlx`. Write integration tests using `testcontainers-rs` (Postgres container).
    * Implement `RabbitMqEventBus` adapter (Publisher/Subscriber) using `lapin`. Write integration tests using `testcontainers-rs` (RabbitMQ container).
    * Implement `RedisCache` adapter using `redis-rs`. Write integration tests using `testcontainers-rs` (Redis container).
    * Implement `RedisEventBus` adapter (for Pub/Sub notifications) using `redis-rs`. Write integration tests using `testcontainers-rs`.
    * Ensure selection between real and in-memory adapters is configurable (e.g., via feature flags or runtime configuration).

6. **Implement DB Logic in Projections & Notifications (Backend - `apps/projection-worker`):** *(Renumbered)*
    * Set up database connection pool (`sqlx::PgPool`) in `projection-worker`.
    * Run database migrations on startup.
    * Update projection handlers (`handle_tenant_created`, `handle_user_registered`) to perform actual DB INSERT/UPDATE operations using `sqlx`.
    * Implement Redis Pub/Sub notification publishing from projection handlers using the `RedisEventBus` adapter.

7. **Implement API Query Endpoints & Caching (Backend - `apps/api-gateway`):** *(Renumbered)*
    * Develop API query endpoints (`GET /api/tenants`, `GET /api/users`).
    * Implement query handlers/logic to read directly from read models using `sqlx::PgPool` (requires adding DB pool to `AppState`).
    * Implement caching for query endpoints using the `RedisCache` adapter (requires adding Cache to `AppState`).

8. **Implement WebSocket Logic (Backend - `apps/api-gateway`):** *(Renumbered)*
    * Configure Axum WebSocket endpoint (`/api/ws`).
    * Implement connection handling, authentication, and subscription logic using the `RedisEventBus` subscriber functionality.
    * Forward messages from Redis Pub/Sub to connected WebSocket clients.

9. **Implement Initial Frontend UI & Real-time Updates (Frontend - `apps/web-ui`):** *(Renumbered)*
    * Integrate Headless UI with Tailwind CSS v4.
    * Build React components for MVP features (Login, API Key, Change Pwd, Tenant Create/List, User Reg/List).
    * Implement WebSocket client logic (`react-use-websocket`) to connect and handle real-time updates.
    * Connect components to REST backend endpoints.

10. **Basic PIREP Flow (Backend & Frontend):** *(Renumbered)*
    * Implement `SubmitPIREP` command handler in `api-gateway`.
    * Implement `PIREPSubmitted` projection handler in `projection-worker` (including DB update & Redis notification).
    * Develop PIREP REST API endpoints (`POST/GET /api/pireps`) in `api-gateway`.
    * Create PIREP UI form/view in `web-ui` with real-time updates.

11. **Initial Platform Admin Setup (Backend - `apps/api-gateway`):** *(Renumbered)*
    * Implement startup logic to create initial `PlatformAdmin` user if none exists, logging one-time password.

12. **Docker Deployment Setup:** *(New Step)*
    * Create `Dockerfile` for `apps/api-gateway`.
    * Create `Dockerfile` for `apps/projection-worker`.
    * Create `docker-compose.application.yml` (or similar) to define services for `api-gateway` and `projection-worker`, linking them to the infrastructure services defined in `docker-compose.infra.yml`.
    * Ensure application can be launched using `docker compose -f docker-compose.infra.yml -f docker-compose.application.yml up`.

13. **Testing & Refinement:** *(Renumbered)*
    * Write/update unit tests (using In-Memory adapters).
    * Write/update integration tests (using `testcontainers-rs` adapters).
    * Write/update E2E tests (Playwright) for core UI flows.
    * Refine implementation based on tests.

14. **Documentation:** *(Renumbered)*
    * Update this plan file (`doc/plans/phase-1-plan.md`).
    * Continuously update Memory Bank files (`activeContext.md`, `progress.md`, `systemPatterns.md`).
    * Add necessary code comments and documentation.
