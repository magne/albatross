# Phase 1 — Step 9 Plan: Query Endpoints & Caching

Status: Planned  
Owners: Backend (api-gateway, projection-worker, core-lib integration)  
Version: v1

## 1. Context & Review Findings

Recent verification of prior steps:

- Steps 1–7: Implemented as described (core ES/CQRS, domain model, command handlers, projection worker with DB + notifications, infra adapters, API key lifecycle).
- Step 8 (RBAC) was marked "NEXT" in `phase-1-plan.md` but is in fact implemented:
  - `application/authz.rs` present with `AuthRole`, `Requirement`, `authorize()`.
  - RBAC enforced in tenant creation, user registration, API key endpoints.
  - Middleware enrichment logic for legacy cache entries implemented.
  - User aggregate invariants for role/tenant validated.
  - Missing: Negative RBAC integration tests, unit tests for `authorize()` itself, some duplication of event replay logic across handlers/middleware.

Plan file needs status update to mark Step 8 as DONE.

### Key Improvement Opportunities Before / During Step 9

1. Security:
   - Placeholder password hashing in `register_user.rs` (`hashed_...`). Replace with Argon2 (same crate used for API key hashing) before login support.
2. Event Replay Duplication:
   - Reconstructing a `User` aggregate (middleware, key handlers) re-implements the same `match` on event types. Extract reusable helper (e.g. `load_user_aggregate(repo, user_id) -> Result<User, CoreError>`).
3. Consistency:
   - `parse_role` duplicated logic & string forms. Consider canonicalizing role strings (Protobuf enum mapping).
4. Missing Negative Tests:
   - RBAC forbidden scenarios not yet covered. Add in this step to avoid drift.
5. Expected Version Handling:
   - `save(&stream_id, expected_version, events)` currently passes aggregate.version() (good) but creation handlers use `0` manually; confirm repository enforces optimistic concurrency.
6. Caching Policy Centralization:
   - TTL constants scattered. Introduce config (env or const module).
7. Password Handling:
   - Aggregate currently stores `password_hash` but events intentionally omit it. Align with projection strategy; ensure no accidental logging.
8. Observability:
   - Add structured logging (span fields like `user_id`, `tenant_id`, `role`) around new query endpoints.
9. Error Mapping:
   - Inline mapping patterns repeat; introduce shared mapper for API layer.
10. Query Abstraction:
    - Introduce lightweight `QueryService` or module for users/tenants to avoid DB access logic in route handlers.

These can be incrementally addressed; only (4) is mandatory inside Step 9 scope; others may be backlog or parallel minor refactors.

## 2. Objectives

- Provide read/query endpoints:
  - `GET /api/tenants`
  - `GET /api/users`
- Enforce RBAC scoping for list results.
- Introduce Redis-backed cache for query responses (scoped).
- Add missing negative RBAC integration tests (from Step 8).
- Lay groundwork for future WebSocket invalidation updates (Step 10).
- Maintain ES/CQRS boundaries: queries read from projections only (no aggregate reconstruction).
- Keep implementation simple (TTL + key namespace) before advanced invalidation.

## 3. Scope

In-Scope:
- Axum handlers for two endpoints.
- SQLx queries over existing read models: `tenants`, `users`.
- Optional pagination/minimal filtering.
- Role-based filtering logic.
- Cache-aside implementation with Redis.
- Tests (unit for scoping, integration for caching + RBAC negative).
- Documentation + memory bank updates.

Out of Scope (Deferred):
- Advanced filtering, search, sorting.
- Real-time push (Step 10).
- Fine-grained cache invalidation via pub/sub.
- GraphQL layer.
- User self profile endpoint (can reuse same path later with `GET /api/users/{id}`).

## 4. Requirements & RBAC Matrix

| Endpoint | PlatformAdmin | TenantAdmin | Pilot |
|----------|---------------|-------------|-------|
| GET /api/tenants | All tenants | Only its own tenant | Only its own tenant (or 403 if we decide to restrict) |
| GET /api/users | All users | Users in its tenant | Self only (user record) |

Decision: For MVP simplicity Pilot will receive only self user row (list of length 1); tenant object accessible via `/api/tenants` (single). If complexity: return 403 for `/api/tenants` for Pilot; choose: MVP returns own tenant to simplify UI.

## 5. Assumptions

- Projections are up-to-date enough (eventual consistency accepted).
- DB schema already supports all required fields.
- Redis available (already dependency for auth).
- Response payload sizes small (no immediate pagination scaling risk).

## 6. Data & Query Design

Tables:
- `tenants(tenant_id, name, created_at, updated_at)`
- `users(user_id, tenant_id, username, email, role, created_at, updated_at)`

Queries (SQLx; add `--!` comments for offline preparation if used):
- PlatformAdmin tenants:
  `SELECT tenant_id, name, created_at, updated_at FROM tenants ORDER BY created_at DESC`
- TenantAdmin / Pilot tenants (single):
  `SELECT tenant_id, name, created_at, updated_at FROM tenants WHERE tenant_id = $1`
- PlatformAdmin users:
  `SELECT user_id, tenant_id, username, email, role, created_at, updated_at FROM users ORDER BY created_at DESC`
- TenantAdmin users:
  `SELECT user_id, tenant_id, username, email, role, created_at, updated_at FROM users WHERE tenant_id = $1 ORDER BY created_at DESC`
- Pilot users (self):
  `SELECT user_id, tenant_id, username, email, role, created_at, updated_at FROM users WHERE user_id = $1`

Pagination (Optional):
- Support `?limit=&offset=`; default `limit=50`, max 200.
- Append `LIMIT $n OFFSET $n`.

## 7. Caching Strategy

Pattern: Cache whole JSON result sets per scope.

Key Format:
- Namespace prefix: `q:v1:` (include version for future invalidation).
- Tenants:
  - PlatformAdmin: `q:v1:tenants:all`
  - Tenant / Pilot: `q:v1:tenants:tenant:{tenant_id}`
- Users:
  - PlatformAdmin: `q:v1:users:all:limit:{L}:offset:{O}`
  - TenantAdmin: `q:v1:users:tenant:{tenant_id}:limit:{L}:offset:{O}`
  - Pilot: `q:v1:users:self:{user_id}` (ignore limit/offset)
Include pagination in cache key for lists.

TTL:
- Tenants list & users lists: 30s–60s (choose 45s compromise) (fast-moving entity counts small).
- Self user (Pilot): 60s.
Rationale: Accept slight staleness before WebSocket invalidation.

Invalidation:
- Rely purely on TTL in Step 9.
- Step 10 will add push invalidation events (Redis pub/sub).

Serialization:
- Store pre-serialized JSON bytes (Vec<u8>).
- Common response wrapper style e.g. `{ "data": [...] }`.

## 8. API Response Shapes

Tenants:
```
{
  "data": [
    { "tenant_id": "...", "name": "...", "created_at": "...", "updated_at": "..." }
  ],
  "pagination": { "limit": 50, "offset": 0, "returned": 1 }
}
```

Users:
```
{
  "data": [
    { "user_id": "...", "tenant_id": "...", "username": "...", "email": "...", "role": "TenantAdmin", "created_at": "...", "updated_at": "..." }
  ],
  "pagination": { "limit": 50, "offset": 0, "returned": 1 }
}
```

Pilot self returns `pagination.returned = 1`.

## 9. Design & Components

Add module: `application/query/` (new)
- `mod.rs`
- `tenants.rs`
- `users.rs`

Structs:
- `TenantRow`, `UserRow` (mirroring read models)
- `QueryService { pool: PgPool, cache: Arc<dyn Cache> }`

Functions:
- `fetch_tenants(scope: TenantsScope, limit, offset) -> Result<Vec<TenantRow>, CoreError>`
- `fetch_users(scope: UsersScope, limit, offset) -> Result<Vec<UserRow>, CoreError>`
- Caching inside service:
  1. Build key
  2. `cache.get`
  3. On miss: execute query, serialize, set

Enums:
- `TenantsScope { All, Tenant { tenant_id } }`
- `UsersScope { All { pagination }, Tenant { tenant_id, pagination }, SelfUser { user_id } }`

RBAC Resolution:
- Determine scope from `AuthenticatedUser` before calling service; no service-level RBAC decisions.

Error Handling:
- Map DB errors to `500`.
- Missing tenant for Pilot (if pilot has tenant_id None) => 404 or 403 (choose 404 to avoid role probing).

Instrumentation:
- Add tracing spans: `span!(Level::DEBUG, "query_tenants", scope=?scope, cache_hit=?hit)`
- Same for users.

## 10. Step-by-Step Implementation Plan

1. Housekeeping (Pre-Query)
   - Add unit tests for `authorize()` (RBAC) (retro Step 8 completeness).
   - Add negative integration tests (RBAC) for:
     - TenantAdmin creating user in another tenant (403)
     - Pilot attempting user registration (403)
     - Second unauthenticated PlatformAdmin registration attempt (401/403)
     - Unauthenticated API key generation attempt (401)
   - (Optional quick refactor) Introduce `load_user_events()` helper (skip if time-constrained; backlog if not necessary now).

2. Create Query Module
   - `application/query/mod.rs` (export submodules).
   - Implement `TenantsScope`, `UsersScope`.
   - Implement key generation helper `fn cache_key(resource: &str, scope: &ScopeEnum, pagination: Option<(&u32,&u32)>)`.

3. Add Query Service
   - Holds `PgPool` and `Arc<dyn Cache>`.
   - Implement `get_tenants(...)` and `get_users(...)` with cache-aside logic and TTL constant.

4. Wire Service into App State
   - Extend `AppState` (if exists) or build a new layer; ensure queries can access `PgPool`.
   - If pool not yet in gateway, add creation (env variables for DSN).

5. Implement Handlers
   - `handle_list_tenants(State(app), Extension(ctx))`
     - Determine scope via role.
     - Parse `limit`, `offset`.
     - Call query service, return JSON.
   - `handle_list_users(...)` similarly.

6. Routing
   - Add routes under `/api/tenants` and `/api/users`.
   - Protect with `api_key_auth` middleware (Pilot always requires key).
   - NOTE: Decide if PlatformAdmin can list resources without tenant (yes).

7. Caching
   - TTL constants: `const TTL_LIST_SECONDS: i64 = 45; const TTL_SELF_SECONDS: i64 = 60;`
   - Future invalidation note (document placeholder hook).

8. Tests
   - Unit:
     - Cache key generation edge cases (pagination variations).
   - Integration:
     - Seed: register PlatformAdmin, create tenant, register TenantAdmin, register Pilot.
     - Generate keys for each, then:
       - PA can list all tenants & users (contains multiple)
       - TA list tenants returns 1, users returns subset
       - Pilot list users returns self only
     - Cache hit path:
       - First request miss, second request (within TTL) hit (assert via instrumentation log or temporary internal counter behind feature flag)
     - RBAC negative cases (from Step 8) included here if not already separate.

9. Documentation & Memory Bank
   - Update `phase-1-plan.md` Step 8 -> DONE, Step 9 -> In Progress.
   - Update `activeContext.md` & `progress.md` after completion.
   - Add caching strategy summary to `systemPatterns.md` (Cache-Aside queries).
   - Record decision about Pilot visibility.

10. Review & Cleanup
   - Run `cargo fmt`, `clippy`.
   - Confirm no sensitive data logged.
   - Ensure SQLx offline build (if practiced) passes.

## 11. Test Plan (Expanded)

| Test | Purpose |
|------|---------|
| `authorize_platform_admin_only` | Unit: ensures only PA passes |
| `authorize_self_or_tenant_admin_self` | Unit: self allowed |
| `authorize_self_or_tenant_admin_tenant_admin_same_tenant` | Unit |
| `authorize_self_or_tenant_admin_forbidden_cross_tenant` | Unit |
| Integration: list tenants/users per role | Validate scoping |
| Integration: pilot user listing contains only self | Data isolation |
| Integration: TA cannot see other tenant users | Isolation |
| Integration: caching second call faster/hit (log assertion) | Cache behavior |
| Integration: negative RBAC (forbidden scenarios) | Hardening |

## 12. Definition of Done

- Endpoints functional with RBAC scoping.
- Caching functions with TTL; verified hits in tests (qualitatively if metrics not available).
- Negative RBAC tests present and passing.
- Step 8 plan state updated; memory bank updated.
- No clippy warnings introduced.
- Plan file and new step plan committed.

## 13. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Stale cached data before revocation/invalidation | Minor confusion | Short TTL + future WS invalidation |
| Pagination not implemented correctly creates large responses | Performance | Enforce limit max 200 |
| RBAC leakage due to logic error | Security | Comprehensive negative tests |
| Redis outage makes queries slower | Performance | On cache error, fall back to DB; log warn |
| Inconsistent role strings in cache keys | Cache fragmentation | Centralize role mapping utility |

## 14. Alternatives Considered

1. No caching (simpler, higher DB load) — rejected (we want pattern established for WS step).
2. Fine-grained invalidation via Redis pub/sub now — deferred to Step 10 for complexity reduction.
3. GraphQL endpoint instead of REST — out of current MVP scope.

## 15. Backlog Items After Step 9

- Add user detail endpoint (`GET /api/users/{id}`) with caching.
- Introduce domain events for query invalidation broadcast (Step 10).
- Replace placeholder password hashing in registration path.
- Extract aggregate replay utility.
- Central config for TTLs + feature flags.
- Rate limiting for list endpoints.

## 16. Effort Estimate

- Implementation: 0.5 day
- Tests (incl. missing RBAC negatives): 0.5 day
- Docs + cleanup: 0.25 day

## 17. Open Questions (Resolve Early)

- Return 403 or 200+empty for Pilot listing tenants if no tenant? (Current assumption: pilot always has tenant; if not, 404.)
- Should Pilot see its own tenant details? (Chosen: yes.)
- Expose email in Pilot self response? (Yes for now; revisit privacy later.)

## 18. Action Checklist

- [ ] Add missing RBAC unit + integration tests.
- [ ] Implement query module + service.
- [ ] Wire routes + middleware.
- [ ] Implement caching keys + TTL constants.
- [ ] Add integration tests full matrix.
- [ ] Update plan & memory bank.
- [ ] Review & merge.

---

Prepared for implementation.
