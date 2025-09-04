# Phase 1 — Step 11 Plan: Initial Frontend UI & Real-Time Reactive Layer

Status: Planned  
Owners: Frontend (apps/web-ui) + Minor Backend Enhancements (api-gateway, projection-worker)  
Version: v1

## 1. Objectives

Primary:

- Introduce the initial interactive frontend (React + Tailwind) to exercise existing backend capabilities (tenant/user creation, API key lifecycle, listing).
- Implement a client-side reactive data layer that:
  - Connects to `/api/ws` (API key authenticated).
  - Subscribes to baseline channels (user, tenant).
  - Performs intelligent cache invalidation / refresh of REST queries on relevant events.
- Upgrade backend event forwarding to include an envelope (`event_type`, `ts`) to eliminate fragile channel-based inference.
- Provide minimal UX for:
  - Bootstrap (first PlatformAdmin creation path)
  - API Key display/creation/revocation
  - Tenant creation & list
  - User registration & list (scoped by role)
  - Basic change password form (stub / no-op if backend not yet implemented)
- Establish testing patterns (Vitest) for hooks, query invalidation, and WebSocket event handling.
- Prepare groundwork for future PIREP flow (Step 12) by solidifying state + real-time patterns.

Secondary:

- Add Redis→WS integration test covering new envelope.
- Add observability placeholders (counters/log spans) without full metrics stack.
- Document frontend architectural decisions and reactive pattern in Memory Bank.

Out of Scope (Defer):

- Full authentication (username/password login) session management (API still lacks login endpoint).
- Advanced UI polish, theming system, role-based component-level guards beyond simple checks.
- PIREP submission flow (Step 12).
- Optimistic UI mutations (will rely on post-command cache refetch for now).
- Backpressure and WS reconnection exponential backoff tuning (basic reconnect only).
- Internationalization (i18n).
- Dark mode toggle.

## 2. Current Context

- Backend command & query endpoints for tenants/users/API keys exist; real-time WS (Step 10) completed with baseline JSON frames (no `event_type` yet).
- Frontend currently only has router skeleton (`App.tsx`) with placeholder pages; no state management, no dependencies beyond React/Router.
- API key bootstrap path exists (first PlatformAdmin registration unauthenticated route).
- Query endpoints (`/api/tenants`, `/api/users`) deliver role-scoped results; caching occurs server-side with TTL.
- WS channels: `user:{id}:updates`, `user:{id}:apikeys`, `tenant:{tenant_id}:updates`.
- Need to unify event-driven invalidation to avoid manual stale decisions.

## 3. Scope Summary

In-Scope:

- Add dependencies: `@tanstack/react-query`, optional `react-use-websocket` OR custom minimal WS hook (choose path below).
- Implement API client abstraction & auth context (API key oriented).
- Resource hooks (`useTenants`, `useUsers`, `useUserSelf`, `useUserApiKeys`).
- WebSocket connection manager + subscription registry.
- Event envelope addition (backend projection publisher & WS forwarder).
- Mapping: events → query invalidation and/or targeted query data patch.
- UI pages & components (foundational CRUD and key management).
- Test suite (Vitest) for hooks & event-driven invalidation.
- Integration test (Rust) verifying Redis publish → WS frame with envelope.

Out-of-Scope:

- Password-based auth flow & token/session storage.
- Multi-tab cross-window sync (can rely on reload).

## 4. Assumptions & Uncertainties

Assumptions:

- API key endpoints stable.
- WebSocket server supports query `?api_key=` or bearer header (already implemented).
- Envelope addition won&#39;t break existing tests (will update/extend them).
- React Query acceptable new dependency (license/perf OK).

Uncertainties (Resolve Early):

1. Should we use `react-use-websocket` or custom?  
2. Do we enrich all Redis publications at projection worker level or wrap only at gateway?  
3. Will we need per-event partial model patching now or simply invalidate queries?

Proposed Resolutions:

- (1) Implement custom lightweight hook (fewer dependencies; logic is straightforward).
- (2) Enrich at projection-worker before publish so any future consumers benefit; gateway acts as pass-through.
- (3) Start with query invalidation (simpler, reliable) + targeted patch for API key list events (low complexity improvement).

## 5. Approaches (Evaluation)

A. Minimal Custom Hooks + React Context + Manual Fetch  
   Pros: Lowest dependency surface; full control.  
   Cons: Re-implements caching, dedupe, stale handling.

B. React Query for server state + Custom WS Hook for invalidation (CHOSEN)  
   Pros: Mature cache, straightforward invalidation, dev tools, retry logic.  
   Cons: Adds dependency; slight learning overhead.

C. Zustand Store + WS → Store + Manual REST Layer  
   Pros: Unified store, fine-grained updates.  
   Cons: Must hand-roll fetch consistency & stale logic; more boilerplate.

Chosen: B — quickest path to a robust & evolvable system.

## 6. Backend Envelope Enhancement

Modify projection worker publish step:

Current publish payload (example): `{ ...event_json }`  
New payload shape:

```json
{
  "event_type": "UserRegistered",
  "ts": "2025-08-12T10:15:30.123Z",
  "data": { ...domain_projection_payload... },
  "meta": {
    "tenant_id": "uuid-or-null",
    "aggregate_id": "user_id",
    "version": 3
  }
}
```

Redis channel unchanged (e.g., `user:{user_id}:updates`).  
Gateway WS forwarder wraps into frame:

```json
{
  "type": "event",
  "channel": "user:123:updates",
  "payload": {
    "event_type": "UserRegistered",
    "ts": "...",
    "data": { ... },
    "meta": { ... }
  }
}
```

Backward compatibility: Not required (frontend not yet consuming).  
Tests updated to assert presence of `event_type`.

## 7. Frontend Architecture (React Router v7 + Data APIs)

We will leverage React Router v7 data routers (route objects with `loader` / `action`) for initial data acquisition and route-based revalidation, integrating with React Query for caching, mutation utilities, and WS-driven invalidation. Strategy: Router loaders perform initial fetch (SSR-friendly in future), seed React Query cache, and UI components consume via React Query hooks to unify subsequent refetch logic.

Layers:

- `routes/` — Route definitions (tree) using createBrowserRouter (or `<RouterProvider/>`) with loader functions returning dehydrated data (tenants/users/apiKeys). Each loader writes to React Query cache using a shared helper (so components can rely solely on queries).
- `api/` — Thin fetch wrappers returning JSON (auto attaches API key).
- `auth/` — `ApiKeyProvider` (Context) storing current API key + derived role/user/tenant state (populated using bootstrap or self/user list loader on first navigation).
- `ws/` — `useRealtime` hook: connect, baseline subscriptions, reconnection (linear backoff 1s→5s), dispatch events.
- `state/` — React Query (QueryClient provider) + Router hydration boundary; future SSR can dehydrate.
- `hooks/` — Resource queries (`useTenants`, `useUsers`, `useUserSelf`, `useUserApiKeys`) — wrappers around `useQuery` with stable keys; optionally expose `prefetchX` used inside loaders.
- `realtime/mapper.ts` — Channel + event_type → invalidation or patch logic; can also trigger `router.revalidate()` for routes whose loader keys are tied to invalidated queries (belt & suspenders).
- `components/` — UI forms & tables.
- `pages/` — Route-level components focusing on layout + composition (thin, leaning on hooks).
- `routes.config.ts` — Central export of route tree (for testability).

Directory sketch (updated):

```text
src/
  api/
    client.ts
    tenants.ts
    users.ts
    apikeys.ts
  auth/
    ApiKeyContext.tsx
  realtime/
    useRealtime.ts
    mapper.ts
    types.ts
  state/
    queryClient.ts
    router.tsx
    preload.ts (helpers to seed query cache from loader)
  hooks/
    useTenants.ts
    useUsers.ts
    useUserSelf.ts
    useUserApiKeys.ts
  routes/
    index.tsx (root layout + loader)
    tenants.tsx (loader + page)
    users.tsx (loader + page)
    apikeys.tsx (loader + page)
    dashboard.tsx (loader minimal)
  components/
    TenantList.tsx
    TenantCreateForm.tsx
    UserList.tsx
    UserCreateForm.tsx
    ApiKeyPanel.tsx
    BootstrapAdminForm.tsx
    ChangePasswordForm.tsx (stub)
  pages/
    Dashboard.tsx
    TenantsPage.tsx
    UsersPage.tsx
    ApiKeysPage.tsx
  App.tsx (mounts RouterProvider + QueryClientProvider + ApiKeyProvider + useRealtime)
```

Loader Pattern:

1. Loader receives `request` & context (future).
2. Reads needed API resources using `api/*`.
3. Seeds React Query cache:

    ```ts
    // preload.ts
    export async function seedQuery<T>(
    qc: QueryClient,
    key: QueryKey,
    fetcher: () => Promise<T>
    ) {
    const existing = qc.getQueryData<T>(key);
    if (!existing) {
        const data = await fetcher();
        qc.setQueryData(key, data);
        return data;
    }
    return existing;
    }
    ```

4. Loader returns minimal shape `{ dehydratedKeys: [...] }` (optional) — components ignore loader data and rely on queries (ensures single source of truth).

Route Revalidation:

- React Router revalidation triggers (navigations, mutation actions) can be complemented by calling `router.revalidate()` after significant mutations if immediate refresh desired even before WS event arrives.
- WS invalidation: After invalidating relevant React Query keys, if certain route-level data is exclusively loader-driven (edge case), call `router.revalidate()` to ensure any loader-only state updates (should be rare once queries unify state).

Benefits:

- Progressive enhancement path to SSR or static prefetch.
- Unified caching (React Query) with route-based initial data load for fast first paint.
- Minimizes duplication (no separate global prefetch layer).

Rationale vs Only React Query:

- Router loaders offer structured composition + future SEO/SSR readiness.
- React Query retains fine-grained cache + invalidation; we avoid double-fetch by seeding cache in loaders.

Open Consideration:

- For Step 11 we keep loaders simple; later we can adopt `defer()` for streaming large collections or PIREP tables.

## 8. Data Flow & Reactive Invalidation

Flow:

1. Component mounts → React Query fetches cached or remote data.
2. WebSocket connects with API key; baseline subscriptions auto-sent.
3. Server pushes event → `useRealtime` dispatches to mapper.
4. Mapper logic:
   - Parse channel pattern:
     - `user:{uid}:updates` → invalidate `user_self` if matches current user OR invalidate `users` list if role is admin and that user is in list.
     - `user:{uid}:apikeys` → invalidate / patch `user_api_keys`.
     - `tenant:{tid}:updates` → invalidate `tenants` list and maybe `tenant:{tid}` (future detail endpoint).
   - If event_type among known minor changes (e.g., `ApiKeyGenerated`) and its data structure includes full latest key list (if we decide), we patch; else simple invalidation.
5. React Query re-fetches on next render (or immediate `invalidateQueries` triggers refetch).

Algorithm Snippet:

```ts
function handleEvent(evt) {
  const { channel, payload } = evt;
  if (channel.startsWith("user:") && channel.endsWith(":updates")) {
    queryClient.invalidateQueries(["users"]);
    if (isSelf(channel)) queryClient.invalidateQueries(["user_self"]);
  } else if (channel.endsWith(":apikeys")) {
    queryClient.invalidateQueries(["user_api_keys", extractUserId(channel)]);
  } else if (channel.startsWith("tenant:")) {
    queryClient.invalidateQueries(["tenants"]);
  }
}
```

Debounce: Group invalidations in an animation frame micro-queue to avoid burst refetch (optional). Initial implementation may directly invalidate (low volume expected).

## 9. API Client & Auth Handling

- `ApiKeyContext` stores `apiKey`, `userId`, `role`, `tenantId`.
- After setting API key (manual entry or bootstrap response), fetch `/api/users` (role-based) or self endpoint (if added later) to populate context.
- For bootstrap: call `POST /api/users/register` (assuming path) with platform admin details (confirm existing route names; adjust if needed).
- Persist API key in `localStorage` (opt-out for security baseline). Provide "Clear Key" button.

Fetch Wrapper:

```ts
async function apiFetch(path: string, init?: RequestInit) {
  const res = await fetch(import.meta.env.VITE_API_BASE + path, {
    ...init,
    headers: {
      ...(init?.headers || {}),
      Authorization: apiKey ? `Bearer ${apiKey}` : undefined,
      'Content-Type': 'application/json'
    }
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}
```

## 10. Components (MVP)

- `BootstrapAdminForm`: Only shown if no API key set; inputs: username, email, password (placeholder), submits to bootstrap endpoint.
- `ApiKeyPanel`: List existing keys + generate + revoke (display masked key; show once on generation).
- `TenantCreateForm`: Visible for PlatformAdmin only.
- `TenantList`: Displays tenant name(s).
- `UserCreateForm`: Different role options depending on creator role (PlatformAdmin can create TenantAdmin & Pilot; TenantAdmin can create Pilot; constraint logic).
- `UserList`: Table of users (scoped).
- `ChangePasswordForm`: Placeholder (submits to endpoint or shows "Not yet implemented").
- `Dashboard`: Quick overview (counts, current user).
- Layout navigation guards by role.

## 11. WebSocket Hook Design

`useRealtime` responsibilities:

- Manage connection life cycle.
- Expose status: `connected`, `attempt`, `lastError`.
- Auto-subscribe baseline after open.
- Accept programmatic subscriptions (future) via returned API.
- Perform keep-alive (server already sends heartbeat; respond to `ping` or ignore if server handles).
- Reconnect on close codes except normal (1000).

Pseudo:

```ts
export function useRealtime() {
  const { apiKey } = useApiKey();
  useEffect(() => {
    if (!apiKey) return;
    let ws = new WebSocket(`${API_WS_BASE}/api/ws?api_key=${apiKey}`);
    ws.onmessage = (m) => dispatch(JSON.parse(m.data));
    // reconnect logic...
    return () => ws.close();
  }, [apiKey]);
}
```

## 12. Testing Strategy (Frontend)

Vitest + React Testing Library:

| Test | Purpose |
|------|---------|
| `mapper/user_update_invalidation.test.ts` | Ensures correct queries invalidated |
| `useRealtime/reconnect.test.ts` | Simulate close → reconnect |
| `apiKeyContext/persist.test.ts` | API key stored/cleared |
| `components/ApiKeyPanel.test.tsx` | Generate/revoke flows call API client |
| `integration/invalidation_roundtrip.mock.test.ts` | Mock WS events lead to refetch (mock fetch counters) |

Mock WS using `WebSocket` polyfill injection & programmatic event dispatch.

Backend (Rust) Integration (new):

- Start Redis ("testcontainers"), start gateway & projection worker minimal, publish synthetic event (using enriched format), assert client test script (maybe using `tokio_tungstenite`) receives envelope.

## 13. Security & Hardening Considerations

- API key stored in `localStorage` only after explicit user action; warning banner about security.
- Avoid logging full API keys in frontend console.
- WebSocket reconnection stops after N (e.g., 10) consecutive failures (fail-closed) to prevent infinite thrash.
- Sanitize/validate envelope fields (defensive check on `event_type` string, ignore unknown).

## 14. Observability Placeholders

Frontend:

- Simple in-memory counter of events processed (dev overlay maybe).
Backend:
- Log event envelope at trace level only.
- Add debug log: `ws_event_forwarded {channel, event_type}`.

## 15. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Over-invalidation causing extra load | Performance | Debounce invalidations; refine mapping |
| Reconnection storm if server down | Redis / server pressure | Backoff + cap attempts |
| Event envelope drift | Client breakage | Version field optional (add if needed) |
| Race between command response & event invalidation | Stale flash in UI | React Query staleTime=0 ensures quick refresh |
| API key leakage (UI copy) | Security | Mask display after generation, require explicit reveal |

## 16. Alternatives Considered

1. Use `react-use-websocket` (Expanded Alternative)  
   - Pros: Built-in reconnect (with configurable backoff), heartbeat / ping-pong helpers, automatic JSON parsing option, cleaner hook signature (`useWebSocket(url, { onMessage, shouldReconnect })`).  
   - Cons: Extra dependency (bundle size + maintenance), less fine-grained control over low-level lifecycle (e.g., custom jitter strategies), still need custom channel/event mapping + invalidation integration.  
   - Adoption Criteria (When to Switch From Custom Hook):  
     - Emergence of more advanced reconnection policies (exponential + jitter) needed.  
     - Need for binary frame support or compression if library updates ahead of custom code.  
     - Desire for standardized event throttling built-in rather than bespoke logic.  
   - Integration Impact:  
     - Replace custom `useRealtime` with wrapper around `useWebSocket`.  
     - Provide small adapter layer that normalizes `lastMessage` into internal `dispatchEvent(envelope)` pipeline.  
     - Keep mapper + React Query invalidation unchanged.  
   - Migration Path:  
     - Phase A: Implement internal interface (`RealtimeClient` abstraction) used by components.  
     - Phase B: Swap implementation from custom to `react-use-websocket` behind that interface.  
   - Decision for Step 11: Defer adding dependency until complexity requires it; proceed with custom minimal implementation but structure code (interface + adapter) to allow drop-in.  

2. Patch queries directly instead of invalidation  
   - Pros: Lower server load, immediate UI updates, potential for optimistic concurrency synergy later.  
   - Cons: Need per-event schema mapping; premature complexity for MVP; introduces risk if envelope evolution outpaces patch logic.  
   - Deferred Until: Event envelope stabilizes and high event volume makes naive invalidation costly.

3. Use SSE instead of WS for read-only  
   - Pros: Simplicity (one-way stream), native browser support, lighter protocol overhead.  
   - Cons: Lacks bi-directional control (future dynamic subscriptions, client-origin messages), harder future feature extension (e.g., targeted subscription negotiation).  
   - Rejected for long-term flexibility.

4. Global Polling with Short TTL (Control)  
   - Pros: Zero WS complexity, predictable load.  
   - Cons: Stale windows, higher aggregate request volume, poor scalability, defeats real-time objective.  
   - Rejected (only a fallback for catastrophic WS outages).

Chosen path: Custom minimal hook (extensible) + React Query invalidation; document upgrade path to `react-use-websocket` to minimize future refactor friction.

## 17. Detailed Implementation Steps

Backend (Envelope):

1. Update projection worker publisher: wrap existing event JSON with envelope.
2. Add `event_type`, `ts` (UTC), `meta`.
3. Adjust WS forwarder: now forwards envelope untouched inside `payload`.
4. Add integration test (Redis → WS frame has envelope).

Frontend:
5. Add dependencies: `@tanstack/react-query`.
6. Create `QueryClientProvider` wrapper in `main.tsx`.
7. Implement `ApiKeyContext` + provider with persistence.
8. Implement `api/client.ts` + specific resource modules.
9. Implement resource hooks using React Query keys:
    - `['tenants']`
    - `['users']`
    - `['user_self']`
    - `['user_api_keys', userId]`
10. Implement `useRealtime` hook & event dispatcher.
11. Implement `realtime/mapper.ts`:
    - Parse channel + event_type → call `invalidateQueries`.
12. Build UI components & pages (scaffold quickly).
13. Wire routes + navigation structure (Dashboard default).
14. Add placeholder change password form (disabled).
15. Add tests (unit then component).
16. Run `pnpm biome:check` & fix formatting.
17. Update docs (`phase-1-plan.md` Step 11 → In Progress) after acceptance.
18. Update Memory Bank (`activeContext.md`, `progress.md`, `systemPatterns.md`) with:
    - New reactive client pattern
    - Event envelope pattern
19. Commit incremental changes logically (separate commits: backend envelope, frontend infra, components, realtime, tests, docs).

## 18. Query Key & Invalidation Matrix

| Channel | Event Type (examples) | Invalidate Keys | Patch? |
|---------|-----------------------|-----------------|--------|
| user:{id}:updates | UserRegistered, ChangePasswordCompleted (future) | ['users'], ['user_self'] if self | Maybe later |
| user:{id}:apikeys | ApiKeyGenerated, ApiKeyRevoked | ['user_api_keys', id] | Possible direct patch later |
| tenant:{id}:updates | TenantCreated | ['tenants'] | N |

(Envelope event_type canonical list defined gradually.)

## 19. Definition of Done

- Backend publishes enriched envelope; WS frames include `event_type` & `ts`.
- Frontend can:
  - Enter/set API key (or perform bootstrap for first admin).
  - List tenants/users per RBAC scope.
  - Create tenant (PlatformAdmin) & user (per role rules).
  - Generate & revoke API keys; UI updates after events without manual refresh.
- Real-time events invalidate relevant queries (verify manually + automated test).
- React Query integrated; no stale UI after changes and event delivery.
- Vitest suite covers mapper + core hooks.
- Integration test (backend) passes for envelope flow.
- Memory Bank updated; plan files updated.
- No linter errors; builds succeed.

## 20. Test Plan (Expanded)

Backend (Rust):

| Test | Description |
|------|-------------|
| ws_event_envelope_forward | Publish synthetic envelope; client receives full structure |
| ws_event_type_presence | Assert non-empty event_type |

Frontend (Vitest):

| Test | Description |
|------|-------------|
| mapper_tenant_update | Triggers tenants invalidation |
| mapper_user_apikeys_update | Invalidates user_api_keys |
| realtime_reconnect | Reconnect attempts with capped backoff |
| api_key_context_storage | Persists and clears API key |
| query_invalidation_roundtrip | Simulated WS event increments refetch count |

Manual QA:

- Create tenant; verify UI list updates with no manual reload.
- Generate API key; panel updates.
- Revoke API key; panel updates.

## 21. Effort Estimate

- Backend envelope + test: 0.25 day
- Frontend infra (query client, auth, ws): 0.5 day
- Components & forms: 0.75 day
- Testing (frontend + backend): 0.5 day
- Docs / Memory Bank: 0.25 day
Total: ~2.25 days

## 22. Open Questions (Answer Before Implement)

1. Are existing command routes for user/tenant creation stable naming? (Assume yes; adapt if mismatch during implementation.)
2. Include simple "copy API key" button? (Yes.)
3. Should we display raw event stream in dev panel? (Optional dev-only; add if time.)

Defaults applied if no contrary instruction.

## 23. Action Checklist

- [ ] Approve plan
- [ ] Implement envelope (backend)
- [ ] Add React Query
- [ ] Build auth + API client layers
- [ ] Build resource hooks
- [ ] Implement realtime hook + mapper
- [ ] Implement UI components/pages
- [ ] Add tests (backend + frontend)
- [ ] Update docs & memory bank
- [ ] Final review & commit

---

Prepared for approval.
