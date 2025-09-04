import { useQuery } from '@tanstack/react-query'
import { useApi } from '../api/client'

export function useUsers(limit = 50, offset = 0) {
  const api = useApi()
  return useQuery({
    queryKey: ['users', limit, offset],
    queryFn: () => api.listUsers(limit, offset),
    staleTime: 5 * 60 * 1000 // 5 minutes
  })
}

export function useUserSelf() {
  const api = useApi()
  return useQuery({
    queryKey: ['user_self'],
    queryFn: () => api.listUsers(1, 0).then((res) => res.data[0] || null), // Assuming self is first or need a dedicated endpoint
    staleTime: 5 * 60 * 1000
  })
}
