import { QueryClient } from '@tanstack/react-query'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { handleEventDispatch } from './mapper'

describe('handleEventDispatch', () => {
  let queryClient: QueryClient
  let invalidateQueriesSpy: any

  beforeEach(() => {
    queryClient = new QueryClient()
    invalidateQueriesSpy = vi.spyOn(queryClient, 'invalidateQueries')
  })

  it('should invalidate tenants on tenant updates', () => {
    const event = {
      type: 'event',
      channel: 'tenant:123:updates',
      payload: {}
    }

    handleEventDispatch(event, queryClient, 'user1')

    expect(invalidateQueriesSpy).toHaveBeenCalledWith({ queryKey: ['tenants'] })
  })

  it('should invalidate users on user updates', () => {
    const event = {
      type: 'event',
      channel: 'user:456:updates',
      payload: {}
    }

    handleEventDispatch(event, queryClient, 'user1')

    expect(invalidateQueriesSpy).toHaveBeenCalledWith({ queryKey: ['users'] })
  })

  it('should invalidate user_self if current user', () => {
    const event = {
      type: 'event',
      channel: 'user:user1:updates',
      payload: {}
    }

    handleEventDispatch(event, queryClient, 'user1')

    expect(invalidateQueriesSpy).toHaveBeenCalledWith({ queryKey: ['users'] })
    expect(invalidateQueriesSpy).toHaveBeenCalledWith({ queryKey: ['user_self'] })
  })

  it('should invalidate user_api_keys on apikeys updates', () => {
    const event = {
      type: 'event',
      channel: 'user:456:apikeys',
      payload: {}
    }

    handleEventDispatch(event, queryClient, 'user1')

    expect(invalidateQueriesSpy).toHaveBeenCalledWith({ queryKey: ['user_api_keys', '456'] })
  })

  it('should do nothing for non-event types', () => {
    const event = {
      type: 'heartbeat',
      channel: 'tenant:123:updates',
      payload: {}
    }

    handleEventDispatch(event, queryClient, 'user1')

    expect(invalidateQueriesSpy).not.toHaveBeenCalled()
  })

  it('should do nothing for unknown channels', () => {
    const event = {
      type: 'event',
      channel: 'unknown:123:updates',
      payload: {}
    }

    handleEventDispatch(event, queryClient, 'user1')

    expect(invalidateQueriesSpy).not.toHaveBeenCalled()
  })
})
