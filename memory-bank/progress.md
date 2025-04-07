# Progress

* **Current Status:** Phase 0 (Foundation & Setup) completed. Project is ready to enter Phase 1 (Core MVP).
* **Completed Features/Milestones (Phase 0):**
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
  * Updated Memory Bank files to reflect current state.
* **Work In Progress:** None. Ready for Phase 1.
* **Upcoming Work (Phase 1 Start):**
  * Implement core ES/CQRS plumbing in Axum service(s) (command handling, aggregate loading/saving, event publishing).
  * Develop initial Aggregates/Events (e.g., Airline, User/Pilot, PIREP) using Protobuf.
  * Implement PostgreSQL Event Store logic (append, read stream, optimistic concurrency).
  * Implement basic Projection Worker logic (consuming RabbitMQ events, updating PostgreSQL read models).
  * Design and implement initial PostgreSQL Read Models.
  * Develop API endpoints in Axum for core MVP features.
  * Implement basic Redis caching and Pub/Sub notifications.
  * Build frontend UI components for MVP features.
* **Known Issues/Bugs:** None specific yet.
  * *Potential Risks:* Inherent complexity of ES/CQRS and microservices. Managing schema evolution. Ensuring robust multi-tenancy isolation. Operational overhead of chosen stack (especially if self-hosting infra in K8s).
* **Decision Log:** (Summary of key decisions from initial planning & recent updates)
  * **Project Name:** Albatross (Finalized for now).
  * **Architecture:** ES/CQRS, Microservices (planned), Multi-tenant.
  * **Backend Stack:** Rust / Axum framework, PostgreSQL, RabbitMQ, Redis.
  * **Frontend Stack:** React, React Router, Vite (with SWC), Tailwind CSS v4.
  * **Infrastructure Stack ("Scenario B"):** PostgreSQL (Events/Reads), RabbitMQ (Event Bus), Redis (Cache/PubSub).
  * **Repo Structure:** Monorepo (Cargo Workspace, PNPM).
  * **Serialization:** Protobuf (using `prost`, stored as binary `bytea`).
  * **Deployment:** Support 3 models (Single Executable, Docker Compose, Kubernetes/k3s).
  * **Licensing:** Dual AGPLv3+Commercial or BSL model preferred over standard OSI licenses due to commercial restrictions requirement.
  * **Infrastructure Management:** Separate reusable definitions (Docker Compose files, Helm Charts) from application code.
  * **Linting/Formatting:** Biome (JS/TS/JSON), cargo fmt/clippy (Rust).
