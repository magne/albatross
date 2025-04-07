# Progress

* **Current Status:** Phase 1, Steps 1 & 2 completed. Ready to start Step 3 (Command Handlers).
* **Completed Features/Milestones:**
  * **Phase 1, Steps 1 & 2:**
    * Defined core ES/CQRS Ports (traits) in `libs/core-lib`.
    * Implemented in-memory adapters (`InMemoryEventRepository`, `InMemoryEventBus`, `InMemoryCache`) in `libs/core-lib`.
    * Defined Protobuf messages for `Tenant`, `User`, and `PIREP` commands/events in `libs/proto`.
    * Implemented initial Aggregate roots (`Tenant`, `User`, `Pirep`) in `libs/core-lib/src/domain/`.
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
* **Work In Progress:** None.
* **Upcoming Work (Phase 1, Step 3 Start):**
  * Implement command handlers in `apps/api-gateway` for `RegisterUser`, `ChangePassword`, `GenerateApiKey`, `LoginUser`, `CreateTenant`.
  * Implement basic command dispatch logic in `api-gateway`.
  * Ensure handlers use the appropriate Ports (`Repository`, `EventPublisher`).
  * (Subsequent Steps): Implement PostgreSQL Event Store logic, Projection Worker logic, Read Models, API endpoints, Redis caching/PubSub, Frontend UI, Testing.
* **Known Issues/Bugs:** None specific yet.
  * *Potential Risks:* Inherent complexity of ES/CQRS and microservices. Managing schema evolution. Ensuring robust multi-tenancy isolation. Operational overhead of chosen stack (especially if self-hosting infra in K8s). Password handling in aggregates needs careful review (currently placeholder).
* **Decision Log:** (Summary of key decisions from initial planning & recent updates)
  * Project Name: Albatross (Finalized for now).
  * Architecture: ES/CQRS, Hexagonal (Ports & Adapters), Microservices (planned), Multi-tenant.
  * Backend Stack: Rust / Axum framework, PostgreSQL, RabbitMQ, Redis.
  * Frontend Stack: React, React Router, Vite (with SWC), Tailwind CSS v4, Headless UI.
  * Infrastructure Stack ("Scenario B"): PostgreSQL (Events/Reads), RabbitMQ (Event Bus), Redis (Cache/PubSub).
  * Repo Structure: Monorepo (Cargo Workspace, PNPM).
  * Serialization: Protobuf (using `prost`, stored as binary `bytea`).
  * Deployment: Support 3 models (Single Executable uses In-Memory Adapters).
  * Licensing: Dual AGPLv3+Commercial or BSL model preferred over standard OSI licenses due to commercial restrictions requirement.
  * Infrastructure Management: Separate reusable definitions (Docker Compose files, Helm Charts) from application code.
  * Linting/Formatting: Biome (JS/TS/JSON), cargo fmt/clippy (Rust).
  * Initial Setup: Platform Admin created on first run with logged one-time password.
  * UI Components: Headless UI chosen.
  * Real-time: WebSockets included in Phase 1 MVP.
  * Migrations: `refinery` crate chosen.
