import { useQuery } from '@tanstack/react-query'
import { useApiKey } from '../state/ApiKeyContext'

export function useUserApiKeys(userId?: string) {
  const { userId: currentUserId } = useApiKey()
  const targetUserId = userId || currentUserId
  return useQuery({
    queryKey: ['user_api_keys', targetUserId],
    queryFn: async () => {
      if (!targetUserId) return []
      const API_BASE = (import.meta.env.VITE_API_BASE || 'http://localhost:3000').replace(/\/$/, '')
      const res = await fetch(`${API_BASE}/api/users/${targetUserId}/apikeys/list`, {
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${localStorage.getItem('apiKey') || ''}`
        }
      })
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const data = await res.json()
      return data.data || []
    },
    enabled: !!targetUserId,
    staleTime: 5 * 60 * 1000 // 5 minutes
  })
}
