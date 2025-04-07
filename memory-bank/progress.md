# Progress

* **Current Status:** Initial planning phase completed (as documented in `doc/conversations/2025-04-05-initial.md`). Core architecture, technology stack, deployment models, and initial development plan defined. Project is ready to enter Phase 0 (Foundation & Setup).
* **Completed Features/Milestones:**
  * Initial requirements discussion and analysis.
  * Technology stack evaluation and selection (Axum, Tailwind, Postgres, RabbitMQ, Redis).
  * Architectural pattern selection (ES/CQRS, Microservices planned, Multi-tenant).
  * Definition of 3 deployment models.
  * Repository structure decision (Monorepo).
  * Licensing model discussion.
  * Creation of revised development plan (Phased approach).
  * Initialization and update of Memory Bank files based on planning conversation.
* **Work In Progress:** None. Currently between planning and Phase 0 execution.
* **Upcoming Work:** Phase 0 tasks:
  * Finalize Frontend Framework (Decision Postponed).
  * Set up Monorepo structure (Git, Cargo Workspace, JS Workspace).
  * Create initial service/app skeletons and shared libraries.
  * Set up infrastructure via Docker Compose (`infrastructure.yml`).
  * Create initial Helm charts for infrastructure.
  * Set up basic CI/CD pipeline.
  * Integrate Protobuf build process.
* **Known Issues/Bugs:** None specific yet.
  * *Potential Risks:* Inherent complexity of ES/CQRS and microservices. Managing schema evolution. Ensuring robust multi-tenancy isolation. Operational overhead of chosen stack (especially if self-hosting infra in K8s).
* **Decision Log:** (Summary of key decisions from initial planning & recent updates)
  * **Project Name:** Albatross (Finalized for now).
  * **Architecture:** ES/CQRS, Microservices (planned), Multi-tenant.
  * **Backend:** Rust / Axum framework.
  * **Frontend:** Vite build tool, Tailwind CSS styling. Framework TBD (React/Vue/Svelte - Decision Postponed). Headless UI/Radix recommended.
  * **Infrastructure Stack ("Scenario B"):** PostgreSQL (Events/Reads), RabbitMQ (Event Bus), Redis (Cache/PubSub).
  * **Repo Structure:** Monorepo.
  * **Serialization:** Protobuf (using `prost`, stored as binary `bytea`).
  * **Deployment:** Support 3 models (Single Executable, Docker Compose, Kubernetes/k3s).
  * **Licensing:** Dual AGPLv3+Commercial or BSL model preferred over standard OSI licenses due to commercial restrictions requirement.
  * **Infrastructure Management:** Separate reusable definitions (Docker Compose files, Helm Charts) from application code.
