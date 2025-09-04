import { useQuery } from '@tanstack/react-query'
import { createContext, type ReactNode, useCallback, useContext, useEffect, useState } from 'react'

export interface ApiKeyContextValue {
  apiKey: string | null
  userId: string | null
  role: string | null
  tenantId: string | null
  setApiKey: (k: string | null) => void
  setUserContext: (ctx: { userId: string; role: string; tenantId: string | null }) => void
  clear: () => void
}

const ApiKeyContext = createContext<ApiKeyContextValue | undefined>(undefined)

const UserContextLoader = ({ children }: { children: ReactNode }) => {
  const { apiKey, setUserContext } = useApiKey()

  // Direct API call to get user self data
  const { data: userData, isSuccess } = useQuery({
    queryKey: ['user_self'],
    queryFn: async () => {
      if (!apiKey) return null
      const API_BASE = (import.meta.env.VITE_API_BASE || 'http://localhost:3000').replace(/\/$/, '')
      const res = await fetch(`${API_BASE}/api/users/list?limit=1&offset=0`, {
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${apiKey}`
        }
      })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const data = await res.json()
      return data.data?.[0] || null
    },
    enabled: !!apiKey,
    staleTime: 5 * 60 * 1000 // 5 minutes
  })

  useEffect(() => {
    if (isSuccess && userData && apiKey) {
      console.log('Setting user context:', userData)
      setUserContext({
        userId: userData.user_id,
        role: userData.role,
        tenantId: userData.tenant_id
      })
    }
  }, [userData, apiKey, setUserContext, isSuccess])

  return <>{children}</>
}

export const ApiKeyProvider = ({ children }: { children: ReactNode }) => {
  const [apiKey, setApiKeyState] = useState<string | null>(null)
  const [userId, setUserId] = useState<string | null>(null)
  const [role, setRole] = useState<string | null>(null)
  const [tenantId, setTenantId] = useState<string | null>(null)

  // load persisted key
  useEffect(() => {
    const stored = localStorage.getItem('apiKey')
    if (stored) {
      setApiKeyState(stored)
    }
  }, [])

  const setApiKey = useCallback((k: string | null) => {
    setApiKeyState(k)
    if (k) {
      localStorage.setItem('apiKey', k)
    } else {
      localStorage.removeItem('apiKey')
    }
  }, [])

  const setUserContext = useCallback((ctx: { userId: string; role: string; tenantId: string | null }) => {
    setUserId(ctx.userId)
    setRole(ctx.role)
    setTenantId(ctx.tenantId)
  }, [])

  const clear = useCallback(() => {
    setApiKey(null)
    setUserId(null)
    setRole(null)
    setTenantId(null)
  }, [setApiKey])

  const value: ApiKeyContextValue = {
    apiKey,
    userId,
    role,
    tenantId,
    setApiKey,
    setUserContext,
    clear
  }

  return (
    <ApiKeyContext.Provider value={value}>
      <UserContextLoader>{children}</UserContextLoader>
    </ApiKeyContext.Provider>
  )
}

export function useApiKey() {
  const ctx = useContext(ApiKeyContext)
  if (!ctx) {
    throw new Error('useApiKey must be used within ApiKeyProvider')
  }
  return ctx
}
