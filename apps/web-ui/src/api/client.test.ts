import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderHook } from '@testing-library/react'
import React from 'react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiKeyProvider } from '../state/ApiKeyContext'
import { type BootstrapStatusResponse, useApi } from './client'

// Mock fetch globally
const fetchMock = vi.fn()
globalThis.fetch = fetchMock

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false }
    }
  })
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(
      QueryClientProvider,
      { client: queryClient },
      React.createElement(ApiKeyProvider, null, children)
    )
}

describe('API Client', () => {
  beforeEach(() => {
    fetchMock.mockClear()
  })

  describe('checkBootstrapStatus', () => {
    it('should call the correct endpoint without authentication', async () => {
      const mockResponse: BootstrapStatusResponse = { needs_bootstrap: true }
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockResponse)
      })

      const { result } = renderHook(() => useApi(), {
        wrapper: createWrapper()
      })

      const response = await result.current.checkBootstrapStatus()

      expect(fetchMock).toHaveBeenCalledWith('http://localhost:3000/api/bootstrap/status', {
        headers: {
          'Content-Type': 'application/json'
        }
      })
      expect(response).toEqual(mockResponse)
    })

    it('should handle successful response with needs_bootstrap: false', async () => {
      const mockResponse: BootstrapStatusResponse = { needs_bootstrap: false }
      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockResponse)
      })

      const { result } = renderHook(() => useApi(), {
        wrapper: createWrapper()
      })

      const response = await result.current.checkBootstrapStatus()

      expect(response.needs_bootstrap).toBe(false)
    })

    it('should handle HTTP errors', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 500,
        json: () => Promise.resolve({ error: 'Internal server error' })
      })

      const { result } = renderHook(() => useApi(), {
        wrapper: createWrapper()
      })

      await expect(result.current.checkBootstrapStatus()).rejects.toThrow('HTTP 500')
    })

    it('should handle network errors', async () => {
      fetchMock.mockRejectedValueOnce(new Error('Network error'))

      const { result } = renderHook(() => useApi(), {
        wrapper: createWrapper()
      })

      await expect(result.current.checkBootstrapStatus()).rejects.toThrow('Network error')
    })
  })
})
