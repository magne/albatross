# Active Context

* **Current Focus:** Initial project planning and setup based on the conversation log (`doc/conversations/2025-04-05-initial.md`). Finalizing foundational decisions and preparing for Phase 0 implementation.
* **Recent Changes:** (Based on initial planning conversation)
  * Defined core project goal: Multi-tenant VA Management Platform.
  * Selected core architecture: ES/CQRS, Microservices (planned), Multi-tenant.
  * Selected technology stack:
    * Backend: Rust / Axum
    * Frontend: Vite + Tailwind CSS + (React/Vue/Svelte TBD, Headless UI/Radix suggested)
    * Infrastructure: PostgreSQL (Events/Reads), RabbitMQ (Event Bus), Redis (Cache/PubSub) - "Scenario B".
  * Decided on Monorepo structure.
  * Outlined 3 Deployment Models (Single Executable, Docker Compose, Kubernetes/k3s).
  * Recommended Protobuf for event/command serialization.
  * Discussed licensing options (Dual AGPLv3+Commercial or BSL).
  * Revised initial development plan into phases.
* **Next Steps:** Execute Phase 0 of the revised plan:
  * Finalize Frontend Framework (Decision Postponed).
  * Set up Monorepo (Git, Cargo Workspace, potentially JS workspace).
  * Create initial service/app skeletons and shared libraries.
  * Set up infrastructure via Docker Compose (`infrastructure.yml`).
  * Create initial Helm charts for infrastructure.
  * Set up basic CI/CD pipeline.
  * Integrate Protobuf build process.
* **Active Decisions:**
  * Project Name: Albatross (Finalized for now).
  * Architecture: ES/CQRS, Microservices, Multi-tenant.
  * Stack: Axum, Tailwind, Vite, Postgres, RabbitMQ, Redis.
  * Structure: Monorepo.
  * Deployment: 3 Models defined.
  * Serialization: Protobuf (stored as binary `bytea`).
  * *Pending Final Decision:* Frontend Framework (React/Vue/Svelte - Decision Postponed).
* **Key Patterns/Preferences:**
  * Prioritize Open Source components and minimal vendor lock-in.
  * Aim for good Developer Experience (DX), including debugging support for microservices potentially running outside k3s.
  * Maintain clear separation between application logic and reusable infrastructure definitions.
* **Learnings/Insights:**
  * Analyzed trade-offs for backend/frontend frameworks, component libraries, event stores, multi-tenancy strategies, deployment costs, licensing, and repo structures.
  * Established the feasibility of the 3 deployment models with careful abstraction.
  * Recognized the complexity introduced by microservices, especially for Model 1 deployment.
