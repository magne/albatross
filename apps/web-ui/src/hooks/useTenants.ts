import { useQuery } from '@tanstack/react-query'
import { useApi } from '../api/client'

export function useTenants() {
  const api = useApi()
  return useQuery({
    queryKey: ['tenants'],
    queryFn: () => api.listTenants(),
    staleTime: 5 * 60 * 1000 // 5 minutes
  })
}
