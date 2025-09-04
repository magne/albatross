import type { QueryClient } from '@tanstack/react-query'

interface WsEventEnvelope {
  type: string
  channel?: string
  payload?: unknown
}

/**
 * Map incoming WS event to React Query invalidations.
 * Keep logic small; extend as canonical event_type list grows.
 */
export function handleEventDispatch(evt: WsEventEnvelope, qc: QueryClient, currentUserId: string | null) {
  if (evt.type !== 'event' || !evt.channel) return
  const ch = evt.channel
  // Channel patterns
  if (ch.startsWith('tenant:') && ch.endsWith(':updates')) {
    qc.invalidateQueries({ queryKey: ['tenants'] })
  } else if (ch.startsWith('user:') && ch.endsWith(':updates')) {
    qc.invalidateQueries({ queryKey: ['users'] })
    if (currentUserId && ch === `user:${currentUserId}:updates`) {
      qc.invalidateQueries({ queryKey: ['user_self'] })
    }
  } else if (ch.startsWith('user:') && ch.endsWith(':apikeys')) {
    const userId = ch.split(':')[1]
    qc.invalidateQueries({ queryKey: ['user_api_keys', userId] })
  }
}
