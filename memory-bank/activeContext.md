# Active Context

* **Current Focus:** Phase 1, Step 10 (WebSocket Real-Time Delivery) completed. Backend real-time infrastructure in place. Preparing for Step 11 (Frontend reactive integration & cache invalidation leveraging real-time events) and incremental hardening (integration tests for WS + enriching event envelopes).
* **Recent Changes (Step 10 Completion):**
  * Added WebSocket endpoint `/api/ws` (Axum) with authentication via API key (Bearer header or `?api_key=` query).
  * Implemented connection lifecycle: baseline auto-subscriptions (user updates, user apikeys, tenant updates), dynamic subscribe/unsubscribe, ping/pong, heartbeat (30s), idle timeout (90s), per-connection rate limiting (10 control messages / 10s).
  * Added Redis Pub/Sub forward loop (baseline subscriptions) – forwards events as `{"type":"event","channel":...,"payload":...}` frames.
  * Implemented message protocol: `ack`, `event`, `error`, `pong`, `heartbeat`.
  * Added channel validation (user self-only; tenant scoped) with unit tests.
  * Wired optional `redis_client` into `AppState`; main process reads `REDIS_URL`.
  * Added tests for rate limiter & channel validation (unit). (Integration tests for end-to-end Redis→WS still pending.)
  * Updated dependency manifest (Axum ws feature, futures-util, tokio-stream, redis).
  * Ensured cargo tests pass after adapting existing test `AppState` initializations (added `redis_client: None`).
* **Recent Changes (Step 9 Completion – Query Endpoints & Caching):**
  * Implemented `GET /api/tenants/list` and `GET /api/users/list` with RBAC-scoped filtering (PlatformAdmin → all; TenantAdmin → own tenant; Pilot → self + own tenant).
  * Added Redis cache keys (`q:v1:...`) with TTL differentiation.
  * Added integration tests (`query_routes.rs`) covering role scoping, unauthorized access, caching consistency, negative RBAC paths (tenant admin cross-tenant create, pilot create).
  * Added pagination normalization and key generation strategy.
* **Earlier Recent Changes (Step 8 RBAC Recap):**
  * RBAC module, enforcement across key endpoints, bootstrap rules, API key operations authorization.
* **System State Summary:**
  * Command side: Aggregates & handlers for users, tenants, API keys operational.
  * Projection pipeline: Worker persists read models & publishes Redis notifications (events not yet envelope-rich).
  * Query side: Read model queries + caching in place.
  * Real-time: WebSocket streaming baseline established (Step 10).
  * Tests: Unit + integration for commands, queries, RBAC, API key lifecycle; WS has unit tests (channel validation, rate limit); needs integration test for actual Redis publish flow.
* **Next Steps (Planned):**
  1. Step 11: Frontend integration consuming `/api/ws`:
     * Implement client subscription manager and optimistic cache invalidation.
     * Map channels → frontend store keys; reconcile projection refresh triggers.
  2. Enhance WebSocket event envelope (add `event_type`, `ts`) – align with future projection/publisher changes.
  3. Add integration tests: spawn Redis, publish synthetic channel events, assert client receipt.
  4. Backpressure & connection metrics (observability foundation).
  5. Security hardening: PlatformAdmin extended subscription policy (optional future).
* **Active Decisions:**
  * Continue using per-connection Redis Pub/Sub (optimize later).
  * Defer event envelope enrichment to post-integration (keeps Step 10 scope bounded).
  * Keep WebSocket JSON-only (binary & compression deferred).
* **Risks / Mitigations:**
  * Scaling Redis connections → future multiplex design.
  * Missing `event_type` → short-term client heuristic by channel → mitigate by envelope work soon.
  * Lack of WS integration tests → scheduled early in next step to prevent regressions.
* **Backlog / Deferred (from Step 10 Plan):**
  * Shared multiplexer for channels.
  * Envelope with `event_type`.
  * Extended admin subscription rules.
  * Replay/snapshot request message type.
  * Observability metrics & compression.
* **Key Patterns Extended:**
  * Introduced Real-time Delivery pattern: Redis Pub/Sub → API WS forwarder → Client reactive layer (pending).
* **Open Items to Track:**
  * Add negative RBAC tests for API key misuse scenarios (if any remaining).
  * Evaluate graceful shutdown of WS tasks (currently relies on task termination via errors/close).
  * Consider structured error codes enumeration for frontend alignment.
* **Learnings / Insights (New):**
  * Axum 0.8 WebSocket `Message::Text` expects `Utf8Bytes` – require `.into()` on `String`.
  * Splitting WebSocket requires Arc<Mutex<SplitSink>> when multiple tasks send frames.
  * Test servers needed adjustment after `AppState` shape change (redis_client).
  * Unit tests inside ws module compiled but not auto-run in `--quiet` grouping due to path—still executed (0 test modules for some crates).
* **Current Focus Transition:** Prepare Step 11 plan (frontend reactive integration) while adding WS integration tests & event envelope improvement as near-term tasks.
