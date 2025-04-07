# Phase 1 Plan: Core MVP & ES/CQRS Foundation (v5)

This plan outlines the steps for implementing the Minimum Viable Product (MVP) features for the Albatross Virtual Airline Management Platform, focusing on establishing the core Event Sourcing (ES) / Command Query Responsibility Segregation (CQRS) loop, Hexagonal Architecture, basic authentication, and real-time UI updates via WebSockets. It includes provisions for in-memory adapters for testing and single-executable deployment.

**MVP Features:**

* Platform Admin initial setup.
* Tenant (Airline) Creation (by Platform Admins).
* User Registration (PlatformAdmin, TenantAdmin, Pilot roles).
* User Login & Password Management.
* API Key Generation/Management.
* Basic PIREP Submission & Viewing.
* Real-time UI updates for relevant changes.

**Technology Stack Highlights:**

* Backend: Rust, Axum, PostgreSQL (Event Store & Read Models), RabbitMQ (Event Bus), Redis (Cache & Pub/Sub), `prost` (Protobuf), `refinery` (Migrations).
* Frontend: TypeScript, React, Vite (SWC), Tailwind CSS v4, Headless UI, `react-use-websocket`.
* Architecture: ES/CQRS, Hexagonal (Ports & Adapters), Microservices (planned, starting with `api-gateway` & `projection-worker`), Multi-tenant.

**Plan Steps:**

1. **Core ES/CQRS, Hexagonal Infra & WebSockets (Backend - `libs/core-lib`, `apps/api-gateway`, `apps/projection-worker`):**
    * Define core **Ports** (Rust traits) in `libs/core-lib` for `Aggregate`, `Command`, `Event`, `Repository`, `CommandHandler`, `EventHandler`, `EventPublisher`, `EventSubscriber`, `Cache`.
    * Implement **Adapters**:
        * `PostgresEventRepository` adapter (implementing `Repository` port) for PostgreSQL Event Store logic.
        * `InMemoryEventRepository` adapter (implementing `Repository` port) for testing & Model 1 deployment.
        * `RabbitMqEventPublisher` and `RabbitMqEventSubscriber` adapters (implementing `EventPublisher`/`EventSubscriber` ports).
        * `InMemoryEventBus` adapter (implementing `EventPublisher`/`EventSubscriber` ports) using e.g., `tokio::sync::broadcast` for testing & Model 1 deployment.
        * `RedisCache` adapter (implementing `Cache` port).
        * `InMemoryCache` adapter (implementing `Cache` port) using e.g., `dashmap` or `moka` for testing & Model 1 deployment.
        * Axum-specific adapters in `api-gateway` for handling HTTP requests and routing to command handlers/query handlers (Input Ports).
    * Configure Axum in `api-gateway` to handle WebSocket connections (`axum::extract::ws`).
    * Set up Command Dispatcher in `api-gateway`.
    * Create the separate `apps/projection-worker` service. Implement basic event consumption logic using the appropriate `EventSubscriber` adapter (RabbitMQ or InMemory based on config/features).
    * Set up `refinery` for database migrations for Event Store and Read Model schemas. Define initial migration files.
    * Ensure selection between real (Redis, RabbitMQ, Postgres) and in-memory adapters is configurable (e.g., via feature flags or runtime configuration) to support different deployment models and testing scenarios.

2. **Define Initial Domain Model (Backend - `libs/proto`, `libs/core-lib`):**
    * Define Protobuf messages in `libs/proto` for Commands and Events:
        * `Tenant`: `CreateTenant` (Command), `TenantCreated` (Event).
        * `User`:
            * Add `Role` enum (`PlatformAdmin`, `TenantAdmin`, `Pilot`).
            * `RegisterUser` (Command - potentially assigns default role), `UserRegistered` (Event - includes role).
            * `AssignRole` (Command), `RoleAssigned` (Event) - *Optional, maybe role set at registration*.
            * `ChangePassword` (Command), `PasswordChanged` (Event).
            * `GenerateApiKey` (Command), `ApiKeyGenerated` (Event).
            * `LoginUser` (Command), `UserLoggedIn` (Event).
        * `PIREP`: `SubmitPIREP` (Command), `PIREPSubmitted` (Event).
    * Implement Aggregate roots (`Tenant`, `User`, `PIREP`) in `libs/core-lib` (the core domain, independent of infrastructure). User aggregate manages roles and password changes.

3. **Implement Initial Command/Event Flows (Backend - `apps/api-gateway`, `libs/core-lib`):**
    * Implement `RegisterUser` command handler (assigns default role, e.g., Pilot, if not specified).
    * Implement `ChangePassword` command handler.
    * Implement `GenerateApiKey` command handler.
    * Implement `LoginUser` command handler.
    * Implement `CreateTenant` command handler, ensuring it checks if the caller has the `PlatformAdmin` role.
    * Ensure handlers use the `Repository` port to load/save aggregates and the `EventPublisher` port to publish events.

4. **Implement Initial Projections & Notifications (Backend - `apps/projection-worker`):**
    * Design initial Read Model schemas (PostgreSQL tables) for `tenants`, `users` (including role, hashed password/API key), `pireps`. Include `tenant_id` where applicable (not necessarily for platform admins). Use `refinery` to manage schema.
    * Implement Projection Worker logic within `apps/projection-worker` to consume relevant events (`TenantCreated`, `UserRegistered`, `PasswordChanged`, `ApiKeyGenerated`, `PIREPSubmitted`, etc.) via the `EventSubscriber` port.
    * Implement logic to update corresponding read model tables.
    * After successfully updating a read model, publish a notification message to a relevant Pub/Sub channel (using the `EventPublisher` port/adapter - e.g., `tenant:{tenant_id}:updates`, `user:{user_id}:updates`). The message should indicate the type of change or the affected entity.

5. **Implement API & WebSocket Endpoints & Auth (Backend - `apps/api-gateway`):**
    * Develop REST API endpoints:
        * User Registration (`POST /api/users`).
        * Login (`POST /api/auth/login`).
        * API Key Generation (`POST /api/users/me/api-key`).
        * Change Password (`PUT /api/users/me/password`).
        * Create Tenant (`POST /api/tenants` - requires PlatformAdmin role).
        * Querying (`GET /api/tenants`, `GET /api/users`, `GET /api/pireps` - apply role/tenant filtering as needed).
    * Develop WebSocket endpoint (e.g., `/api/ws`). Handle connection authentication (e.g., using API key passed during handshake).
    * Implement logic within the WebSocket handler to subscribe connected clients to appropriate Pub/Sub channels (via the `EventSubscriber` port/adapter) based on their authenticated context (user ID, tenant ID).
    * Forward messages received from Pub/Sub to the relevant connected WebSocket clients.
    * Implement authentication middleware (API Key/Session check) for REST endpoints.
    * Implement authorization logic within handlers or middleware to check for required roles (e.g., `PlatformAdmin` for creating tenants).
    * Implement basic caching for REST queries using the `Cache` port/adapter.

6. **Implement Initial Frontend UI & Real-time Updates (Frontend - `apps/web-ui`):**
    * Integrate Headless UI with Tailwind CSS v4.
    * Build React components using Headless UI/Tailwind for:
        * Login Form.
        * View API Key.
        * Change Password Form.
        * Tenant Creation Form (visible only to PlatformAdmins).
        * View Tenants/Airlines.
        * User Registration Form.
        * View Users.
    * Implement WebSocket client logic (e.g., using `react-use-websocket`) to connect to the backend `/api/ws` endpoint after authentication.
    * Handle incoming WebSocket messages. Update UI state accordingly (e.g., refresh lists, show notifications) when relevant changes are pushed from the backend.
    * Connect components to REST backend, handle auth, conditionally render UI based on user role.

7. **Basic PIREP Flow (Backend & Frontend):**
    * Implement `SubmitPIREP` command handler.
    * Implement Projection Worker logic for `PIREPSubmitted` (including Pub/Sub notification).
    * Develop REST API endpoints (`POST/GET /api/pireps`) - ensure protected by auth (e.g., Pilot role).
    * Create UI form/view for PIREPs (using Headless UI), ensuring the list updates in real-time via WebSockets.

8. **Initial Platform Admin Setup (Backend - `apps/api-gateway` or dedicated init logic):**
    * Implement logic that runs on application startup (e.g., in `api-gateway`'s `main.rs` or via a command-line flag).
    * This logic checks if any `PlatformAdmin` user exists in the read model.
    * If not, it generates a secure one-time password, creates the initial `PlatformAdmin` user (e.g., username "admin") by dispatching a `RegisterUser` command internally (or directly creating the event), and logs the username and one-time password.

9. **Testing & Refinement:**
    * Write unit tests (using `InMemoryEventRepository`, `InMemoryEventBus`, `InMemoryCache`).
    * Write integration tests (covering auth/roles, REST endpoints, WebSocket message broadcasting/receiving, potentially using test containers or mocked adapters for external dependencies).
    * Write E2E tests (Playwright) for core flows including login, password change, tenant creation (as PlatformAdmin), PIREP submission, and real-time updates.
    * Refine based on tests.

10. **Documentation:**
    * Update this file (`doc/plans/phase-1-plan.md`).
    * Continuously update Memory Bank files (`activeContext.md`, `progress.md`, `systemPatterns.md`) as tasks are completed.
    * Add necessary code comments and documentation (e.g., Rust doc comments).
