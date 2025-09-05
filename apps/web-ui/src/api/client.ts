import { useApiKey } from '../state/ApiKeyContext'

const API_BASE = (import.meta.env.VITE_API_BASE || 'http://localhost:3000').replace(/\/$/, '')

export interface ApiError extends Error {
  status: number
  body?: unknown
}

async function request<T>(path: string, init: RequestInit = {}, apiKey?: string | null): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...(apiKey ? { Authorization: `Bearer ${apiKey}` } : {}),
      ...(init.headers || {})
    }
  })
  if (!res.ok) {
    let body: unknown
    try {
      body = await res.json()
    } catch {
      /* ignore */
    }
    const err: ApiError = Object.assign(new Error(`HTTP ${res.status}`), {
      status: res.status,
      body
    })
    throw err
  }
  // Some endpoints (204) have no body
  if (res.status === 204) return undefined as unknown as T
  return (await res.json()) as T
}

// Tenants
export interface TenantDto {
  tenant_id: string
  name: string
  created_at: string
  updated_at: string
}
export interface ListTenantsResponse {
  data: TenantDto[]
  pagination: { limit: number; offset: number; returned: number }
}

// Users
export interface UserDto {
  user_id: string
  tenant_id: string | null
  username: string
  email: string
  role: string
  created_at: string
  updated_at: string
}
export interface ListUsersResponse {
  data: UserDto[]
  pagination: { limit: number; offset: number; returned: number }
}

// API Keys (projection not exposed yet directly; will add when endpoint exists for list)
export interface ApiKeyDto {
  key_id: string
  key_name: string
  created_at: string
}

export interface GenerateKeyResponse {
  key_id: string
  api_key: string
}

export interface CreateTenantResponse {
  tenant_id: string
}

export interface CreateUserResponse {
  user_id: string
}

export function useApi() {
  const { apiKey } = useApiKey()

  return {
    listTenants: () => request<ListTenantsResponse>('/api/tenants/list', {}, apiKey),
    createTenant: (name: string) =>
      request<CreateTenantResponse>(
        '/api/tenants',
        {
          method: 'POST',
          body: JSON.stringify({ name })
        },
        apiKey
      ),
    listUsers: (limit = 50, offset = 0) =>
      request<ListUsersResponse>(`/api/users/list?limit=${limit}&offset=${offset}`, {}, apiKey),
    createUser: (username: string, email: string, password: string, role: number, tenantId?: string) =>
      request<CreateUserResponse>(
        '/api/users',
        {
          method: 'POST',
          body: JSON.stringify({
            username,
            email,
            password_plaintext: password,
            initial_role: role,
            tenant_id: tenantId
          })
        },
        apiKey
      ),
    changePassword: (userId: string, oldPassword: string, newPassword: string) =>
      request<void>(
        `/api/users/${userId}/change-password`,
        {
          method: 'POST',
          body: JSON.stringify({
            old_password: oldPassword,
            new_password: newPassword
          })
        },
        apiKey
      ),
    generateApiKey: (userId: string, keyName: string) =>
      request<GenerateKeyResponse>(
        `/api/users/${userId}/apikeys`,
        {
          method: 'POST',
          body: JSON.stringify({ key_name: keyName })
        },
        apiKey
      ),
    revokeApiKey: (userId: string, keyId: string) =>
      request<void>(`/api/users/${userId}/apikeys/${keyId}`, { method: 'DELETE' }, apiKey)
  }
}
