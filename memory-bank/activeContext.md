# Active Context

* **Current Focus:** Phase 1, Step 3 - Implement Initial Command/Event Flows (Command Handlers).
* **Recent Changes (Phase 1, Steps 1 & 2 Completion):**
  * Defined core ES/CQRS Ports (traits) in `libs/core-lib`.
  * Implemented in-memory adapters (`InMemoryEventRepository`, `InMemoryEventBus`, `InMemoryCache`) in `libs/core-lib` for testing/Model 1.
  * Defined Protobuf messages for `Tenant`, `User` (with Roles, Auth), and `PIREP` commands/events in `libs/proto`.
  * Implemented initial Aggregate roots (`Tenant`, `User`, `Pirep`) in `libs/core-lib/src/domain/`.
  * Verified `libs/core-lib` and `libs/proto` compile successfully.
  * (Phase 0): Finalized core technology stack, set up monorepo, created skeletons, configured Protobuf build, set up frontend, created infra placeholders, basic CI, basic embedding.
* **Next Steps (Phase 1, Step 3 Start):**
  * Implement command handlers in `apps/api-gateway` for `RegisterUser`, `ChangePassword`, `GenerateApiKey`, `LoginUser`, `CreateTenant`.
  * Implement basic command dispatch logic in `api-gateway`.
  * Ensure handlers use the appropriate Ports (`Repository`, `EventPublisher`).
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
