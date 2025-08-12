# Phase 1 — Step 10 Plan: WebSocket Real-Time Delivery (Backend)

Status: Planned  
Owners: Backend (api-gateway, core-lib integration)  
Version: v1

## 1. Objectives

- Expose a WebSocket endpoint (`GET /api/ws`) on `api-gateway`.
- Authenticate connection using existing API key mechanism (same bearer token).
- Auto-subscribe client to baseline entity channels (own user, own tenant where applicable).
- Support client-driven dynamic subscriptions/unsubscriptions.
- Stream real-time projection notifications (currently published via Redis) to connected clients.
- Provide a minimal, evolvable message protocol (JSON frames).
- Lay groundwork for future fine-grained cache invalidation & frontend reactive UI (Step 11).
- Preserve security (multi-tenant isolation; no cross-tenant leakage).

## 2. Current Context

- Projection worker publishes Redis Pub/Sub notifications:
  - `tenant:{tenant_id}:updates` (TenantCreated)  
  - `user:{user_id}:updates` (UserRegistered)  
  - `user:{user_id}:apikeys` (ApiKeyGenerated / ApiKeyRevoked)
- `RedisEventBus::publish` currently forwards raw payload bytes (event JSON) — loses explicit `event_type`.
- No WebSocket route exists yet; Axum stack already in place.
- API key auth implemented (middleware). Need equivalent logic at handshake (cannot rely on per-route layer once connection is upgraded, must authenticate first).

## 3. Scope

In-Scope:

- New route `/api/ws` (WebSocket upgrade).
- Authentication via `Authorization: Bearer <api_key>` header (or fallback `?api_key=` query param).
- Connection lifecycle management (handshake, auth failure → close with 4401).
- Baseline subscriptions + dynamic subscribe/unsubscribe protocol.
- Message protocol (JSON): control, ack, event, error, ping/pong.
- Heartbeats + idle timeout.
- Graceful shutdown of tasks.
- Basic rate limiting of control messages (in-memory per-connection).
- Tests (unit + integration with Redis testcontainer if feasible).

Out of Scope (Defer / Backlog):

- Shared multiplexer across connections (opt for per-connection PubSub first).
- Pattern subscriptions with server-side filtering.
- Authorization rules for subscribing to arbitrary channels beyond owned context (enforce whitelist for now).
- Binary frames, compression, backpressure metrics.
- Event-type enrichment (will add envelope later by enhancing publisher).

## 4. Assumptions & Uncertainties

Assumptions:

- Redis URL available via `REDIS_URL`.
- Axum workspace dependency can enable `"ws"` feature; if not, add it.
- Event payloads are valid UTF-8 JSON (projection worker serialized via `serde_json`).
- Redis Pub/Sub fan-out volume is low (per-entity channels) — per-connection subscriber acceptable.

Uncertainties (Resolve Early):

- Keep or change Redis publishing format? → Keep raw payload for Step 10 (simplify). Enrich later.
- Need channel pattern consolidation? → Not yet (explicit channel list suffices).
- Ping direction? → Server → Client PING every 30s; expect client PONG or any message to refresh liveness.

## 5. Channel & Subscription Model

Allowed channel name patterns (whitelist):

- `user:{user_id}:updates`
- `user:{user_id}:apikeys`
- `tenant:{tenant_id}:updates`

Baseline auto-subscriptions on connect:

- Always: `user:{self_user_id}:updates`, `user:{self_user_id}:apikeys`
- If tenant_id exists: `tenant:{tenant_id}:updates`

Validation Rules:

- A client may only subscribe to:
  - Its own user channels.
  - Its own tenant channels (TenantAdmin/Pilot).
  - PlatformAdmin may subscribe to any tenant or user channel (future need). Step 10: restrict PA to self + its tenantless state unless explicit expansion needed. (Decision: For MVP keep same restrictions as others except PA has no tenant channel; additional cross-tenant subscription blocked until requirement emerges.)

## 6. Message Protocol (JSON Frames)

All frames use UTF-8 text frames.

Inbound (Client → Server):

```json
{ "type": "subscribe", "channels": ["user:UUID:updates"] }
{ "type": "unsubscribe", "channels": ["user:UUID:apikeys"] }
{ "type": "ping", "id": "optional-correlation" }
```

Outbound (Server → Client):

```json
{ "type": "ack", "action": "subscribe", "channels": [...], "accepted": [...], "rejected": [...] }
{ "type": "ack", "action": "unsubscribe", "channels": [...], "removed": [...], "missing": [...] }
{ "type": "event", "channel": "user:UUID:updates", "payload": { ...original event json... } }
{ "type": "error", "code": "invalid_message", "message": "..." }
{ "type": "pong", "id": "optional-correlation" }
{ "type": "heartbeat", "ts": "...iso8601..." }
```

Error Codes (initial):

- `unauthorized`
- `forbidden_channel`
- `invalid_message`
- `rate_limited`
- `internal`

## 7. Connection Lifecycle & Tasks

Per connection:

- Task A: WebSocket read loop (control frames, client messages).
- Task B: Redis Pub/Sub read loop → forward events.
- Task C: Heartbeat ticker (every 30s send `{"type":"heartbeat"}` + ping).
- Shared state: `Arc<Mutex<Subscriptions>>` storing HashSet of channel strings.
- Shutdown: Any task error triggers shutdown broadcast; gracefully close WS with appropriate code (1000 normal, 1011 internal error).

Timeouts:

- Idle (no inbound or outbound activity) 90s → close (server sends close frame).
- Missed 3 heartbeats (no pong) → close.

## 8. Rate Limiting

Per-connection sliding window:

- Max 10 control messages (subscribe/unsubscribe) per 10s.
- Implementation: VecDeque timestamps; prune on each new message.

On violation:

```json
{ "type":"error","code":"rate_limited","message":"Too many control messages" }
```

Then optionally continue (first offense) or close on repeated offenses (e.g., 3rd → close).

## 9. Security & Authorization

- Authenticate before upgrade acceptance; if failure → respond with HTTP 401 (no upgrade).
- After upgrade, treat connection as authenticated principal; do not allow channel subscription referencing foreign tenant/user unless PlatformAdmin (deferred).
- Reject any channel not passing validation: respond in `ack.rejected`.

## 10. Redis Integration Strategy

Option A (Initial): Each connection creates its own `PubSub` and subscribes/unsubscribes dynamically.

Pros: Simplicity, isolation.  
Cons: Higher connection count to Redis.

Option B (Later): Shared multiplex + internal broadcast.

Chosen: Option A (MVP). Document pivot path.

## 11. Event Envelope Gap & Interim Handling

Current Redis publisher discards `event_type`. We forward raw JSON payload plus channel context (client can infer semantics by channel). Future Step: Wrap publisher payload in an envelope (`{event_type, data}`) for richer client usage → backlog item.

## 12. Implementation Steps

1. Dependencies
   - Ensure `axum` has `"ws"` feature.
   - Add `futures` (if not present) for `StreamExt`.
   - Confirm `redis` crate already included indirectly; if not add to `api-gateway` `Cargo.toml` (version aligned with core-lib).
2. Configuration
   - Add `REDIS_URL` extraction in `main.rs` (optional; if not set fallback to in-memory no-op? For Step 10: require).
   - Construct a `RedisEventBus` (publisher unused for now but created uniformly) or directly create redis::Client for Pub/Sub.
3. State Additions
   - Extend `AppState` with `redis_url: Option<String>` OR `redis_client: Option<redis::Client>`.
4. Route
   - Add `.route("/ws", get(ws_handler))` under `/api`.
5. Handshake & Auth
   - `ws_handler`:
     - Extract API key from `Authorization` or query string.
     - Reuse cache lookup logic (refactor small helper from middleware to avoid duplication).
     - On success: upgrade; on failure: return 401.
6. Connection Actor
   - Split socket into sender/receiver.
   - Build baseline subscriptions; subscribe via PubSub.
   - Spawn:
     - `read_control_loop`
     - `redis_forward_loop`
     - `heartbeat_loop`
   - Use `tokio::select!` to watch join handles; close gracefully.
7. Subscription Management
   - Helper: `subscribe_channels(pubsub, &channels)` and `unsubscribe_channels(...)`.
   - Validation: ensure user/tenant matches context.
8. Message Handling
   - Parse JSON; match `type`; apply logic.
   - Unknown -> error frame.
9. Heartbeats
   - Every 30s send heartbeat + ping (outbound frame).
   - On receiving `ping` from client, respond with `pong`.
   - Track last_activity timestamp; enforce idle timeout.
10. Rate Limiting
    - Implement per connection struct with sliding window.
11. Logging / Tracing
    - Span per connection: fields `conn_id`, `user_id`, `tenant_id`.
    - Debug on subscription changes; warn on errors.
12. Testing
    - (Unit) Channel validation logic.
    - (Unit) Rate limiter.
    - (Integration) With testcontainers Redis:
    - Establish WS connection (auth).
    - Publish test message to subscribed channel via redis client; expect forwarded frame.
    - Subscribe dynamic channel success/failure cases.
    - Idle timeout simulation (set shorter heartbeat in test via feature flag).
13. Documentation
    - Update `phase-1-plan.md` Step 10 → In Progress (once implementation starts).
    - After completion update memory bank (`activeContext.md`, `progress.md`, `systemPatterns.md` — add Real-time Delivery pattern).
14. Cleanup
    - Run `cargo fmt`, `clippy`.
    - Ensure no sensitive key material logged.

## 13. Data Structures (Sketch)

```rust
struct WsConnContext {
    user_id: String,
    tenant_id: Option<String>,
    role: String,
}

struct RateLimiter {
    events: VecDeque<Instant>,
    max: usize,
    window: Duration,
}

struct SubscriptionState {
    channels: HashSet<String>,
}
```

Outbound Event Frame (runtime):

```rust
#[derive(Serialize)]
struct EventFrame<T: Serialize> {
    r#type: &'static str, // "event"
    channel: String,
    payload: T,
}
```

## 14. Test Plan (Detailed)

| Test | Type | Description |
|------|------|-------------|
| channel_validation_self_user | Unit | Only own user accepted |
| channel_validation_foreign_user | Unit | Reject foreign user channel |
| baseline_auto_subscriptions | Integration | On connect subscriptions count matches expectations |
| publish_user_update_forwarded | Integration | Simulated JSON publish reaches client as event frame |
| subscribe_dynamic_allowed | Integration | Subscribe request ack accepted |
| subscribe_dynamic_rejected | Integration | Attempt foreign channel returns ack with rejected |
| rate_limit_exceeded | Unit | 11 subs in 10s triggers error |
| heartbeat_closure | Integration | No pong path triggers close |
| unauthorized_handshake | Integration | Invalid API key returns 401 |

## 15. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Per-connection PubSub scaling | Redis connection pressure | Later multiplex optimization |
| Missing event_type | Harder client logic | Add envelope in future step |
| Accidental channel leakage | Data exposure | Strict whitelist + validation unit tests |
| Blocking operations in loops | Latency | Use async non-blocking only |
| Forgotten unsub on drop | Orphaned subs (minor) | PubSub dropped automatically closes subs |

## 16. Alternatives Considered

1. SSE (Server-Sent Events)  
   - Pros: Simpler.  
   - Cons: Bi-directional control harder; we want future features.  
   - Rejected.

2. Shared Redis subscriber + internal broadcast (mpsc)  
   - Pros: Fewer Redis conns.  
   - Cons: More infra complexity early.  
   - Deferred.

3. Embedding event_type now (modify publisher)  
   - Pros: Richer client UX now.  
   - Cons: Cross-cut change; risk scope creep.  
   - Deferred incremental improvement.

## 17. Definition of Done

- `/api/ws` route authenticates & upgrades.
- Clients receive baseline subscription events.
- Dynamic subscribe/unsubscribe works with ack frames.
- Events published in Redis user/tenant channels flow to client as JSON frames.
- Heartbeats and idle timeout enforced.
- Rate limiting active.
- Tests (unit + integration baseline) pass.
- Memory bank & plan updated.

## 18. Backlog (Post Step 10)

- Add event_type envelope in Redis publisher.
- Shared subscription multiplexer.
- Fine-grained cache invalidation notifications.
- Permission-based wildcard subscriptions for PlatformAdmin.
- Replay / snapshot request message type.
- Observability metrics (connected clients, lag).
- Compression & backpressure strategies.

## 19. Effort Estimate

- Implementation: 0.75 day
- Tests: 0.5 day
- Docs & cleanup: 0.25 day

## 20. Open Questions (Confirm Before Implement)

1. Allow PlatformAdmin to subscribe arbitrarily now? (Default: NO, restrict for simplicity)
2. Need query param fallback for API key? (Yes, add `?api_key=` for browser dev convenience)
3. Accept unauthenticated read-only mode? (No)

(Proceed with defaults above unless instructed otherwise.)

---

Prepared for approval.
