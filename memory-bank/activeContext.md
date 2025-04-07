# Active Context

* **Current Focus:** Phase 1 - Core MVP implementation. Building the essential features (Airline Profile, Pilot Registration, basic PIREP submission/viewing) within the established monorepo structure. Proving the core ES/CQRS loop with the chosen stack.
* **Recent Changes (Phase 0 Completion):**
  * Finalized core technology stack (Axum, React, Vite/SWC, Tailwind v4, Postgres, RabbitMQ, Redis, Protobuf).
  * Set up monorepo structure (`apps/`, `libs/`) with Cargo workspace and PNPM/Biome.
  * Created initial service/app skeletons (`api-gateway`, `core-lib`, `proto`, `web-ui`).
  * Configured basic Protobuf build process within `libs/proto`.
  * Set up frontend project (`web-ui`) with React, Router, Vite, SWC, and Tailwind v4.
  * Created basic infrastructure definition (`docker-compose.infra.yml`) and Helm placeholder (`infra/helm/README.md`).
  * Established basic CI workflow (`.github/workflows/ci.yml`).
  * Configured basic frontend embedding (`rust-embed`) in `api-gateway`.
* **Next Steps (Phase 1 Start):**
  * Implement core ES/CQRS plumbing in Axum service(s) (command handling, aggregate loading/saving, event publishing).
  * Develop initial Aggregates/Events (e.g., Airline, User/Pilot, PIREP) using Protobuf.
  * Implement PostgreSQL Event Store logic (append, read stream, optimistic concurrency).
  * Implement basic Projection Worker logic (consuming RabbitMQ events, updating PostgreSQL read models).
  * Design and implement initial PostgreSQL Read Models.
  * Develop API endpoints in Axum for core MVP features.
  * Implement basic Redis caching and Pub/Sub notifications.
  * Build frontend UI components for MVP features.
* **Active Decisions:**
  * Project Name: Albatross (Finalized for now).
  * Architecture: ES/CQRS, Microservices, Multi-tenant.
  * Backend Stack: Axum (Rust), Postgres, RabbitMQ, Redis.
  * Frontend Stack: React, React Router, Vite (with SWC), Tailwind CSS v4.
  * Structure: Monorepo (Cargo Workspace, PNPM).
  * Deployment: 3 Models defined.
  * Serialization: Protobuf (stored as binary `bytea`).
  * Linting/Formatting: Biome (JS/TS/JSON), cargo fmt/clippy (Rust).
* **Key Patterns/Preferences:**
  * Prioritize Open Source components and minimal vendor lock-in.
  * Aim for good Developer Experience (DX), including debugging support for microservices potentially running outside k3s.
  * Maintain clear separation between application logic and reusable infrastructure definitions.
* **Learnings/Insights:**
  * Analyzed trade-offs for backend/frontend frameworks, component libraries, event stores, multi-tenancy strategies, deployment costs, licensing, and repo structures.
  * Established the feasibility of the 3 deployment models with careful abstraction.
  * Recognized the complexity introduced by microservices, especially for Model 1 deployment.
