import { useQuery } from '@tanstack/react-query'
import { useApiKey } from '../state/ApiKeyContext'

export function useUserApiKeys(userId?: string) {
  const { userId: currentUserId } = useApiKey()
  const targetUserId = userId || currentUserId
  return useQuery({
    queryKey: ['user_api_keys', targetUserId],
    queryFn: () => {
      // TODO: Implement API call when endpoint is available
      // For now, return empty array
      return Promise.resolve([])
    },
    enabled: !!targetUserId,
    staleTime: 5 * 60 * 1000 // 5 minutes
  })
}
