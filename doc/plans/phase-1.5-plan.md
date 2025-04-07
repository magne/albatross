# Phase 1.5 Plan: MVP Refinement & Foundation Hardening

This phase focuses on iterating upon the Phase 1 MVP, addressing key foundational aspects identified in `doc/conversations/thoughts.md`, implementing the MVP UI in Vue and Svelte for comparison, and ensuring the system is robust and deployable via Docker.

**Goals:**

* Implement robust Authentication & Authorization mechanisms.
* Finalize Deployment Scenario 1 (Single Executable with SQLite).
* Create initial Dockerfiles and Helm charts (Scenario 2 & 3 setup).
* Integrate basic Observability (Tracing & Logging).
* Implement MVP UI in Vue and Svelte for framework comparison.

**Key Areas & Tasks:**

1. **Authentication & Authorization:**
    * Implement JWT generation on login (consider libraries like `jsonwebtoken`).
    * Implement JWT validation middleware in `api-gateway`.
    * Implement token refresh mechanism.
    * Implement secure logout (e.g., token blocklist if needed, or rely on short expiry).
    * Implement password reset flow (e.g., email sending, unique token generation/validation).
    * Implement email verification flow.
    * Implement user profile update endpoints (change email, change password).
    * Implement account deletion endpoint and associated data removal logic (GDPR/CCPA).
    * Investigate RBAC libraries (`casbin-rs`, `rbac`) and implement basic role checking for tenant administration tasks.

2. **Deployment Scenario 1 (Single Executable):**
    * Add `rusqlite` dependency (with `bundled`, `modern_sqlite` features).
    * Implement `SqliteEventRepository` adapter in `libs/core-lib`.
    * Implement `SqliteReadModelRepository` (or similar query mechanism) if needed for read models in this scenario.
    * Add feature flags (e.g., `sqlite`, `in_memory_infra`) to `libs/core-lib`, `api-gateway`, `projection-worker` to conditionally compile/use SQLite and in-memory adapters.
    * Refine build process (`cargo build --release --features "single_executable_mode"`) to produce a working single binary.
    * Update `refinery` setup to potentially support SQLite migrations alongside Postgres.

3. **Deployment Scenario 2 & 3 (Docker/Kubernetes Setup):**
    * Create multi-stage `Dockerfile` for `apps/api-gateway`.
    * Create multi-stage `Dockerfile` for `apps/projection-worker`.
    * Update `docker-compose.application.yml` to use the built images.
    * Create basic Helm chart structure in `infra/helm/` for `api-gateway` and `projection-worker`.

4. **Observability (Basic):**
    * Add OpenTelemetry dependencies (`opentelemetry`, `opentelemetry_sdk`, relevant exporters like `opentelemetry-otlp`).
    * Integrate basic tracing context propagation in `api-gateway` (e.g., using `tracing-opentelemetry` layer).
    * Integrate basic tracing in `projection-worker` event handling.
    * Configure basic logging exporter (e.g., stdout exporter for OTLP logs).

5. **Multi-Frontend Implementation (Vue & Svelte):**
    * Set up project skeletons for `apps/web-vue` and `apps/web-svelte` (using `create-vue` and SvelteKit templates respectively).
    * Adjust `api-gateway` static file serving and fallback logic to handle `/react/*`, `/vue/*`, `/svelte/*` paths, potentially serving different `index.html` files.
    * Implement the full MVP UI features (as built for React in Phase 1) using Vue/Composition API and Svelte/SvelteKit.
    * Ensure WebSocket client logic is implemented for real-time updates in both Vue and Svelte versions.

6. **Testing & Refinement:**
    * Add integration tests for new Auth/Authz flows.
    * Add tests for SQLite adapters.
    * Test Docker builds and basic Docker Compose deployment.
    * Perform manual testing and comparison of React, Vue, and Svelte frontends.

7. **Documentation:**
    * Update Memory Bank files (`activeContext.md`, `progress.md`, etc.) throughout this phase.
    * Document Auth flows, Deployment setups, Observability integration.
    * Document findings from frontend framework comparison.

*(Note: This plan assumes Phase 1 is completed first. Tasks within Phase 1.5 can potentially be parallelized where dependencies allow.)*
