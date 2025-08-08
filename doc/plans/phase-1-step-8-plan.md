# Phase 1 — Step 8 Plan: Role-Based Authorization (AuthZ) (v1)

## Objective

Introduce role-based authorization aligned with multi-tenancy to control access to protected endpoints using existing API key authentication. Enforce PlatformAdmin (global), TenantAdmin (tenant-scoped), and Pilot (self-scoped) permissions in the API gateway.

## Non-Goals (Step 8)

- No UI changes.
- No JWT/OIDC yet (API keys only).
- No role-change flows (assignment/elevation).
- No broad endpoint coverage beyond current API-key routes (expand later).

## Current State (Summary)

- API key authentication via middleware `api_key_auth` populates `AuthenticatedUser { user_id, tenant_id }`.
- `User` aggregate already tracks `role: proto::user::Role` and `tenant_id: Option<String>`.
- Read model `users` table stores `role` as string; projections exist.
- API key handlers implemented with cache entries keyed by plain API key.

## Deliverables

- Enriched cached `AuthenticatedUser` with `role`.
- Authorization helper/policy in gateway with unit tests.
- Authorization enforced for API key create/revoke routes.
- Integration tests validating allow/deny matrix and tenant isolation.
- Projection verification ensuring role is persisted consistently.

---

## RBAC Model

### Roles (proto)

- PlatformAdmin (global scope)
- TenantAdmin (tenant scope)
- Pilot (self scope)
- Unspecified (invalid for authorization)

### Semantics

- PlatformAdmin: Full access across all tenants.
- TenantAdmin: Manage users/resources only within their tenant.
- Pilot: Manage own resources only (self-service).

### Multi-tenancy Constraints

- For non-PlatformAdmin, all actions must satisfy `ctx.tenant_id == resource_tenant_id`.

---

## Data & Contracts

### Domain

- `libs/core-lib/src/domain/user.rs`
  - Add getters:
    - `pub fn role(&self) -> Role`
    - `pub fn id(&self) -> &str`
  - Keep existing `tenant_id()`.

### Protobuf / Events

- No change (role already set at registration).

### Read Models

- `apps/projection-worker`:
  - Users table already has `role VARCHAR(50)`.
  - Optional future index for large datasets:

    ```sql
    -- sql
    CREATE INDEX IF NOT EXISTS idx_users_tenant_role ON users(tenant_id, role);
    ```

### Cache Shape

- Extend `AuthenticatedUser`:

  ```rust
  // rust
  #[derive(Clone, Debug, Serialize, Deserialize)]
  pub struct AuthenticatedUser {
      pub user_id: String,
      pub tenant_id: Option<String>,
      pub role: String, // "PlatformAdmin" | "TenantAdmin" | "Pilot"
  }
  ```

- Backward compatibility: Enrich legacy entries (missing `role`) on first use in middleware/handler by reading from `users` read model and re-writing the cache entry.

---

## Gateway AuthZ Design

### New Module

- `apps/api-gateway/src/application/authz.rs`

  ```rust
  // rust
  use axum::http::StatusCode;

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum AuthRole { PlatformAdmin, TenantAdmin, Pilot }

  #[derive(Debug, Clone)]
  pub enum Requirement {
      PlatformAdminOnly,
      SelfOrTenantAdmin { target_user_id: String, target_tenant_id: Option<String> },
      // Extend with resource-based constraints as endpoints grow
  }

  pub fn parse_role(s: &str) -> Option<AuthRole> {
      match s {
          "PlatformAdmin" | "ROLE_PLATFORM_ADMIN" => Some(AuthRole::PlatformAdmin),
          "TenantAdmin"   | "ROLE_TENANT_ADMIN"   => Some(AuthRole::TenantAdmin),
          "Pilot"         | "ROLE_PILOT"          => Some(AuthRole::Pilot),
          _ => None,
      }
  }

  pub fn authorize(
      ctx_user_id: &str,
      ctx_tenant_id: &Option<String>,
      ctx_role: AuthRole,
      req: Requirement,
  ) -> Result<(), StatusCode> {
      match req {
          Requirement::PlatformAdminOnly => {
              if ctx_role == AuthRole::PlatformAdmin { Ok(()) } else { Err(StatusCode::FORBIDDEN) }
          }
          Requirement::SelfOrTenantAdmin { target_user_id, target_tenant_id } => {
              if ctx_role == AuthRole::PlatformAdmin { return Ok(()); }
              if target_user_id == ctx_user_id { return Ok(()); }
              // TenantAdmin must match tenant
              if ctx_role == AuthRole::TenantAdmin && ctx_tenant_id.is_some() && ctx_tenant_id == &target_tenant_id {
                  return Ok(());
              }
              Err(StatusCode::FORBIDDEN)
          }
      }
  }
  ```

### Enforcement Points (Step 8 Scope)

- POST `/api/users/{user_id}/apikeys`
- DELETE `/api/users/{user_id}/apikeys/{key_id}`

Rule for both:

- Allow if:
  - PlatformAdmin, or
  - TenantAdmin where `ctx.tenant_id == target_user.tenant_id`, or
  - Pilot where `path.user_id == ctx.user_id`.
- Otherwise 403 (do not leak resource existence).

### Resource Tenant Resolution

- If `target_user_id != ctx.user_id`:
  - Resolve `target_tenant_id` from read model: `SELECT tenant_id FROM users WHERE user_id = $1`.
  - Pass it to `authorize`.
- Use the gateway’s existing `PgPool` if available; if absent, add `pg_pool: sqlx::PgPool` to `AppState` (same pattern as cache injection).

---

## Middleware & Cache Integration

### api_key_auth

- On cache hit:
  - Try to deserialize `AuthenticatedUser`.
  - If `role` missing, load `role` for `ctx.user_id` from read model and update cache entry in-place before proceeding.
- On cache miss/error: 401 (unchanged).

### GenerateApiKey handler

- When creating `AuthenticatedUser`, populate `role` string from aggregate:

  ```rust
  // rust
  let role_str = format!("{:?}", user.role()); // or map explicitly to "PlatformAdmin" etc.
  let authenticated_user = AuthenticatedUser { user_id: input.user_id.clone(), tenant_id: user.tenant_id().cloned(), role: role_str };
  ```

---

## Projection Worker

- Verify UserRegistered projection writes `users.role` consistently with strings expected by parse_role.
- Add/adjust unit tests to assert role persistence and idempotency.

---

## Testing Strategy

### Unit (Policy)

- Cases:
  - Pilot self: allow.
  - Pilot other (same tenant): deny.
  - TenantAdmin same-tenant other-user: allow.
  - TenantAdmin cross-tenant: deny.
  - PlatformAdmin cross-tenant: allow.

### Integration (Gateway)

- Seed:
  - Tenants: A, B.
  - Users: platformAdmin (no tenant), tenantAdminA (A), pilotA (A), pilotB (B).
- Generate API keys for each; cache entries must include `role`.
- Cases:
  - pilotA (Bearer) -> POST apikeys for pilotA: 200
  - pilotA -> POST apikeys for pilotB: 403
  - tenantAdminA -> POST apikeys for pilotA: 200
  - tenantAdminA -> POST apikeys for pilotB: 403
  - platformAdmin -> POST apikeys for pilotB: 200
  - Mirror above for DELETE revoke.
- Backfill path:
  - Manually create a legacy cache entry without `role`; assert first request enriches and proceeds.

---

## Step-by-Step Work Breakdown

1) Core-lib (Getters) — S

- File: `libs/core-lib/src/domain/user.rs`
- Add:

  ```rust
  // rust
  impl User {
      pub fn id(&self) -> &str { &self.id }
      pub fn role(&self) -> Role { self.role }
  }
  ```

- Add simple unit checks.

2) Gateway: Cache Model — M

- File: `apps/api-gateway/src/application/middleware/auth.rs`
- Update struct with `pub role: String`.
- In `api_key_auth`, implement legacy enrichment path:
  - If deserialization lacks `role` or empty, query read model for user’s role, update cache, continue.

3) GenerateApiKey: Populate Role — S

- File: `apps/api-gateway/src/application/commands/generate_api_key.rs`
- Populate `role` in `AuthenticatedUser` from aggregate state before caching.

4) AuthZ Module — M

- New File: `apps/api-gateway/src/application/authz.rs` with `AuthRole`, `Requirement`, `parse_role`, `authorize`.
- Unit tests covering the matrix.

5) Route Enforcement — M-L

- Files: API-key routes (same as tests reference `apps/api-gateway/tests/api_key_routes.rs`).
- Extract `AuthenticatedUser` from request extensions.
- Resolve `target_tenant_id` (self: reuse ctx; other: query read model).
- Call `authorize(...)`; on `Err`, return 403.

6) Projection Verification — S

- Ensure role string mapping matches `parse_role` expectations.
- Add/adjust unit tests in projection worker.

7) Integration Tests — M

- Extend `apps/api-gateway/tests/api_key_routes.rs` (or add `authz_routes.rs`) with matrix above.

8) Formatting/Linting — S

- `cargo fmt`, `cargo clippy -D warnings`.
- Biome for JS/TS (no changes expected this step).

---

## Risks & Mitigations

- Stale roles in cache after role change: Accept for Step 8; future RoleChanged event will invalidate cache.
- Cross-tenant leakage: Always return 403; avoid existence leaks.
- Resource lookup latency: Single `SELECT` per cross-user action; acceptable. Add index later if needed.

---

## Acceptance Criteria

- Cached `AuthenticatedUser` includes `role` for newly generated keys; legacy entries enriched at first use.
- API-key create/revoke routes enforce RBAC rules and tenant isolation as specified.
- Unit and integration tests pass for allow/deny matrix.
- No DB schema changes required; projection writes `users.role` consistently.

---

## Follow-ups (Beyond Step 8)

- Centralize tenant resolution helpers.
- Add role change flows and RoleChanged event with cache invalidation.
- Expand authorization to additional endpoints (Steps 9–12).
- Consider route-level Tower layers for static permissions once resource lookups are standardized.
