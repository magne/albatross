import { useQuery } from '@tanstack/react-query'
import { useApi } from '../api/client'
import { useApiKey } from '../state/ApiKeyContext'

export function useUsers(limit = 50, offset = 0) {
  const api = useApi()
  return useQuery({
    queryKey: ['users', limit, offset],
    queryFn: () => api.listUsers(limit, offset),
    staleTime: 5 * 60 * 1000 // 5 minutes
  })
}

export function useUserSelf() {
  const { apiKey } = useApiKey()
  return useQuery({
    queryKey: ['user_self'],
    queryFn: async () => {
      const API_BASE = (import.meta.env.VITE_API_BASE || 'http://localhost:3000').replace(/\/$/, '')
      const res = await fetch(`${API_BASE}/api/users/self`, {
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${apiKey}`
        }
      })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      return await res.json()
    },
    enabled: !!apiKey,
    staleTime: 5 * 60 * 1000
  })
}
