# Progress

* **Current Status:** Phase 1, Step 11 (Frontend reactive integration & real-time driven cache invalidation) COMPLETED. Frontend implemented with React Query, WebSocket client, UI components for tenant/user/API key management, and bootstrap flow. Missing items implemented: Tenant/User creation forms, ChangePassword stub, Vitest tests for hooks/mapper, backend envelope integration test. All existing tests pass. Preparing for Phase 1, Step 12 (PIREP submission flow).
* **Completed Features/Milestones:**
  * **Phase 1, Step 10:** WebSocket Real-Time Delivery (Backend - `apps/api-gateway`)
    * Added `/api/ws` endpoint with API key authentication (Bearer / optional query).
    * Implemented baseline auto-subscriptions: `user:{id}:updates`, `user:{id}:apikeys`, `tenant:{tenant_id}:updates`.
    * Dynamic subscribe/unsubscribe with validation, `ack` responses including accepted/rejected or removed/missing.
    * Implemented event forwarding loop from Redis Pub/Sub (baseline channels) producing JSON event frames.
    * Added heartbeat (30s), idle timeout (90s), ping/pong support, per-connection rate limiting (10 control messages / 10s).
    * Added channel validation + rate limiter unit tests inside `ws.rs`.
    * Wired optional `redis_client` in `AppState` + initialization from `REDIS_URL`.
    * Updated test AppState initializations with `redis_client: None`.
    * All tests pass (`cargo test --all`).
    * Deferred: Envelope enrichment (`event_type`), integration test for Redis → WS flow, multiplex optimization.
  * **Phase 1, Step 9:** Query Endpoints & Caching (Backend - `apps/api-gateway`) - **DONE**
    * Implemented RBAC-scoped list endpoints (`/api/tenants/list`, `/api/users/list`).
    * Added Redis cache key strategy with TTLs.
    * Added integration tests covering role scoping, caching, negative RBAC scenarios.
  * **Phase 1, Step 8:** Role-Based Authorization - **DONE**
    * RBAC enforcement across user/tenant operations and API key lifecycle.
  * **Phase 1, Step 7:** API Key Authentication & Management - **DONE**
  * **Phase 1, Step 6:** Projection Worker DB & Notifications - **DONE**
  * **Phase 1, Step 5:** Real Infrastructure Adapters - **DONE**
  * **Phase 1, Step 4:** Projection Worker Skeleton & Migrations - **DONE**
  * **Phase 1, Step 3:** Command Handlers & API Gateway Setup - **DONE**
  * **Phase 1, Step 2:** Domain Model & Protobuf - **DONE**
  * **Phase 1, Step 1:** Core ES/CQRS In-Memory Lib - **DONE**
  * **Phase 0:** Project scaffolding & foundational setup - **DONE**
* **Work In Progress:** None (Step 10 closed; planning Step 11).
* **Upcoming Work (Phase 1, Step 11 - Planned):**
  * Frontend WebSocket client, subscription manager, UI state reconciliation.
  * Client-side mapping of channels -> resource invalidation / refresh strategy.
  * Introduce event envelope upgrade (or schedule Step 12 if scope risk).
  * Add Redis→WS integration test (spawn Redis container, publish synthetic messages, assert client receives event).
  * Add observability placeholders (connection count gauge, subscription metrics).
* **Known Issues / Gaps:**
  * No integration test yet verifying Redis Pub/Sub broadcast to WS client.
  * Event frames lack explicit `event_type` (payload-only inference).
  * No backpressure or send queue bounding; potential risk under bursty events.
  * WS unit tests are co-located; might extract to dedicated test module for clarity.
* **Decision Log (Incremental for Step 10):**
  * Per-connection Redis Pub/Sub accepted for MVP; defer multiplex optimization.
  * JSON frames only; no compression or binary frames initially.
  * Rate limit chosen as 10/10s; adjustable if needed after frontend consumption patterns observed.
* **Metrics to Add (Future):**
  * Connected clients, per-channel subscription counts, dropped messages, rate-limit hits.
* **Confidence:** High for baseline real-time path stability; moderate for production scalability (pending multiplex & metrics).
* **Next Action:** Draft Step 11 plan document (`doc/plans/phase-1-step-11-plan.md`) focusing on frontend reactive layer and event envelope evolution.
