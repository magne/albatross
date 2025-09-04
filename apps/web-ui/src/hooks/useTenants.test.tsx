import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useApi } from '../api/client'
import { useTenants } from './useTenants'

// Mock the API
vi.mock('../api/client', () => ({
  useApi: vi.fn()
}))

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false
      }
    }
  })
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  )
}

describe('useTenants', () => {
  it('should fetch tenants successfully', async () => {
    const mockApi = {
      listTenants: vi.fn().mockResolvedValue({
        data: [{ tenant_id: '1', name: 'Test Tenant' }],
        pagination: { limit: 50, offset: 0, returned: 1 }
      }),
      createTenant: vi.fn(),
      listUsers: vi.fn(),
      createUser: vi.fn(),
      changePassword: vi.fn(),
      generateApiKey: vi.fn(),
      revokeApiKey: vi.fn()
    }
    vi.mocked(useApi).mockReturnValue(mockApi)

    const { result } = renderHook(() => useTenants(), {
      wrapper: createWrapper()
    })

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true)
    })

    expect(result.current.data).toEqual({
      data: [{ tenant_id: '1', name: 'Test Tenant' }],
      pagination: { limit: 50, offset: 0, returned: 1 }
    })
    expect(mockApi.listTenants).toHaveBeenCalledTimes(1)
  })
})
