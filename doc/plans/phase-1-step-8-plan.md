# Phase 1 — Step 8 Plan: Role-Based Authorization (RBAC)

Status: Planned  
Owners: Backend (api-gateway, core-lib, projection-worker)  
Version: v1

## Objectives

- Enforce least-privilege access for all backend routes.
- Encode role and tenant scoping into a single, reusable authorization policy.
- Ensure auth works for both:
  - Bootstrap flows (first API key for a user without prior keys).
  - Normal flows (authenticated requests with cached user context).
- Keep tests green and add new coverage for RBAC.

## Scope

In-scope:

- Role model and invariants in `core-lib`.
- Authorization policies and guards in `apps/api-gateway`.
- Read model and cache updates to include `role`.
- Tests (unit + integration) for allowed/denied matrices.

Out of scope (later steps):

- UI enforcement (frontend).
- Fine-grained, attribute-based access; stick to role + tenant for now.

## Assumptions and Uncertainties

Assumptions:

- Protobuf `Role` enum already includes `PlatformAdmin`, `TenantAdmin`, `Pilot`.
- `User` aggregate stores `role` and `tenant_id` and is reconstructible from events.
- API key cache entries can store serialized `AuthenticatedUser` values.

Uncertainties:

- Whether we require an explicit domain command to change roles in Phase 1 (likely no — initial roles set at registration is sufficient).
- Whether read models already persist user roles; plan for a migration if not.

## Architecture & Design

- Centralize authorization decisions in a small policy module:
  - `Requirement` enum (e.g., `PlatformAdminOnly`, `SelfOrTenantAdmin { target_user_id, target_tenant_id }`).
  - `authorize(ctx_user_id, ctx_tenant_id, ctx_role, req) -> Result<(), StatusCode>`.
- Authn (API key) remains cache-based:
  - Cache value: `{ user_id, tenant_id, role }`.
  - Backward compatibility: if legacy cache entry lacks `role`, rebuild `User` from events and rehydrate cache with role (with TTL).
- Enforcement strategy:
  - Apply middleware for general protection (e.g., `/api/protected`).
  - For routes with path parameters (user/tenant), apply guard in-handler where we have target IDs.
- Bootstrap exception:
  - Allow generating the very first API key for a user without auth only if that user has `api_key_count() == 0`.

## Role Model & Invariants (core-lib)

- `User.role: Role` (Proto-backed enum).
- Invariants:
  - `PlatformAdmin` must not have a `tenant_id`.
  - Non-`PlatformAdmin` must have a `tenant_id`.
- Public getters required:
  - `id() -> &str`, `role() -> Role`, `tenant_id() -> Option<&String>`, `api_key_count() -> usize`.
- No new domain commands for role changes in Phase 1; role is set at registration.

## Read Models & Projections (projection-worker)

- Ensure user read model includes `role` and `tenant_id`.
- Migration if needed:
  - `03__users_add_role_column.sql`:
    - `ALTER TABLE users ADD COLUMN IF NOT EXISTS role INT NOT NULL DEFAULT 0;`
    - Backfill can be omitted for MVP if reads don’t depend on it; gateway can still derive role from events for legacy cache entries.
- Update projection handlers to set `role` on `UserRegistered` and maintain tenant integrity.

## Authorization Policy (api-gateway)

- Introduce `application::authz` module:
  - `AuthRole` (mapped from string or proto): `PlatformAdmin`, `TenantAdmin`, `Pilot`.
  - `parse_role(&str) -> Option<AuthRole>`.
  - `Requirement` enum:
    - `PlatformAdminOnly`
    - `SelfOrTenantAdmin { target_user_id, target_tenant_id: Option<String> }`
  - `authorize(...) -> Result<(), StatusCode>`.
- Policy rules:
  - `PlatformAdminOnly`: only platform admins allowed.
  - `SelfOrTenantAdmin`:
    - Allow if `ctx.user_id == target_user_id`.
    - Or allow if `ctx.role == TenantAdmin` and `ctx.tenant_id == target_tenant_id`.
    - Or allow if `ctx.role == PlatformAdmin`.

## Enforcement Matrix (initial endpoints)

- `POST /api/tenants`: `PlatformAdminOnly`.
- `POST /api/users`: `PlatformAdmin` (any) or `TenantAdmin` (only within own tenant).
- `POST /api/users/{user_id}/apikeys`:
  - `SelfOrTenantAdmin` targeting `user_id`’s tenant; OR bootstrap if user has zero keys.
- `DELETE /api/users/{user_id}/apikeys/{key_id}`:
  - `SelfOrTenantAdmin` targeting `user_id`’s tenant.
- `GET /api/protected`: any authenticated role.

## Backward Compatibility

- Legacy cache entries without `role`: on first use, reconstruct `User` from events, compute `role`, update cache entry with TTL.
- Tests updated to include `Authorization: Bearer {api_key}` where required.

## Telemetry

- Log authorization decisions at `debug`.
- Log denials at `warn` with reason (`forbidden`, `unauthorized`, missing header).
- Avoid logging secrets; only log `key_id` for key operations.

## Risks & Mitigations

- Risk: Over-enforcement breaking existing tests.
  - Mitigation: Add bootstrap path for first API key; update tests to set headers where appropriate.
- Risk: Cache misses causing increased event-store reads during legacy period.
  - Mitigation: Rehydrate cache with TTL to amortize cost.

## Step-by-Step Implementation Plan

1. Core-lib domain (if not present)
   - Add/verify getters on `User`: `id()`, `role()`, `tenant_id()`, `api_key_count()`.
   - Ensure registration invariants enforce tenant constraints for roles.
   - Unit tests: registration with/without tenant for each role; key counting after events.

2. api-gateway authz module
   - Create `application/authz.rs` with:
     - `AuthRole`, `parse_role`, `Requirement`, `authorize()`.
   - Unit tests for `authorize()` positive/negative cases.

3. Middleware enrichment
   - In `application/middleware/auth.rs`:
     - Deserialize `AuthenticatedUser` from cache; if missing `role`, rehydrate from events; update cache with TTL.
   - Test: end-to-end middleware with enriched legacy cache data.

4. Route guards
   - For `POST /api/users/{user_id}/apikeys`:
     - If `Authorization` exists: enforce `SelfOrTenantAdmin` with target tenant derived from aggregate.
     - Else bootstrap: only allow if target user has no existing API keys.
   - For `DELETE /api/users/{user_id}/apikeys/{key_id}`:
     - Require auth; enforce `SelfOrTenantAdmin`.
   - For `POST /api/users` and `POST /api/tenants`:
     - Enforce respective requirements.
   - Integration tests:
     - Generate + revoke with auth.
     - Revoke with wrong tenant admin should be forbidden.
     - Bootstrap generate allowed only once without auth.

5. Projections & migrations
   - If the `users` read model lacks `role`:
     - Add migration `03__users_add_role_column.sql`.
     - Update projection logic to write `role`.
   - Testcontainers-based integration can be deferred if timeboxed; include a TODO and staged plan.

6. Caching contract
   - Define serialized `AuthenticatedUser`:
     - `{ user_id: String, tenant_id: Option<String>, role: String }`.
   - TTL (e.g., 30 days) for new entries and rehydrated entries.

7. Error mapping
   - `401 Unauthorized` for missing/invalid credentials.
   - `403 Forbidden` for valid credentials lacking privileges.
   - Keep existing mapping for core errors.

8. Documentation updates
   - Update `doc/plans/phase-1-plan.md` status when done.
   - Update memory bank: `activeContext.md`, `progress.md`, `systemPatterns.md` with chosen patterns and enforcement matrix.

## Test Plan

- Unit tests:
  - `authz::authorize()` covering all branches.
  - `User` invariants and getters.

- Integration tests (Axum):
  - Happy paths:
    - PlatformAdmin creates tenant; TenantAdmin creates user in same tenant; user self-generates key with auth.
  - Denied paths:
    - TenantAdmin creates user in other tenant.
    - Pilot attempts to create/revoke keys for others.
    - Revoke with missing/invalid auth.
  - Bootstrap:
    - First key generation without auth allowed; second without auth denied.

- If available later: testcontainers
  - Validate projection role persistence and cache enrichment working against real infra.

## Rollout & Feature Flag

- Soft-rollout: keep bootstrap path.
- Optionally guard strict enforcement with an env flag if needed (default: enabled).

## Definition of Done

- All guarded routes pass new RBAC tests.
- Legacy cache entries are auto-enriched with role on first use.
- Migrations (if needed) applied and projections updated.
- Documentation and Memory Bank updated.

## Alternatives Considered

1) Per-route closures for authorization checks  

    - Pros: Simple, explicit.  
    - Cons: Duplicated logic, hard to keep consistent.  
    - Verdict: Not chosen.

2) Tower layers for role-aware middleware per-scope  

    - Pros: Composable; can attach at router subtree.  
    - Cons: Requires request-time access to path params/target entity, which is awkward.  
    - Verdict: Partial use acceptable, but main checks remain in handlers.

3) Attribute macros for declarative policies  

    - Pros: DRY, declarative.  
    - Cons: Complexity overhead for MVP; macro maintenance.  
    - Verdict: Not chosen for Phase 1.

Chosen approach: small policy module + handler-level guards. It’s explicit, testable, and simple.

## Work Breakdown (Tasks)

- core-lib
  - Verify/add getters on `User`.
  - Add/verify invariants for role/tenant at registration.
  - Unit tests for invariants and key counting.

- api-gateway
  - Add `application/authz.rs` with `Requirement` and `authorize()`.
  - Update `application/middleware/auth.rs` to enrich legacy cache entries with role.
  - Enforce guards in:
    - `POST /api/users` (PA or TA)
    - `POST /api/tenants` (PA)
    - `POST /api/users/{user_id}/apikeys` (SelfOrTenantAdmin or bootstrap)
    - `DELETE /api/users/{user_id}/apikeys/{key_id}` (SelfOrTenantAdmin)
  - Update/extend integration tests (`tests/api_key_routes.rs`).

- projection-worker
  - Add migration for `users.role` if missing; update projection handlers to write role.

- docs
  - Update Phase 1 plan and memory bank after completion.

## Estimated Effort

- Implementation: 0.5–1 day
- Tests: 0.5 day
- Docs and cleanup: 0.25 day
