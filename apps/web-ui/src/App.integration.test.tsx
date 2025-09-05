import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import React from 'react'
import { beforeEach, describe, expect, it, type MockedFunction, vi } from 'vitest'
import '@testing-library/jest-dom'
import App from './App'
import { RealtimeProvider } from './realtime/useRealtime'
import { ApiKeyProvider } from './state/ApiKeyContext'

// Mock fetch globally with proper typing
const mockFetch = vi.fn() as MockedFunction<typeof fetch>
globalThis.fetch = mockFetch

// Mock WebSocket globally with proper typing
const MockWebSocket = vi.fn().mockImplementation(() => ({
  readyState: 1, // OPEN
  close: vi.fn(),
  onopen: null,
  onmessage: null,
  onerror: null,
  onclose: null,
  send: vi.fn()
}))

Object.assign(MockWebSocket, {
  CONNECTING: 0,
  OPEN: 1,
  CLOSING: 2,
  CLOSED: 3
})

globalThis.WebSocket = MockWebSocket as unknown as typeof WebSocket

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
      React.createElement(ApiKeyProvider, null, React.createElement(RealtimeProvider, null, children))
    )
}

describe('App Integration Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Clear localStorage
    localStorage.clear()
  })

  describe('Authentication Flow', () => {
    it('should show bootstrap form when no users exist', async () => {
      // Mock bootstrap status - no users exist
      mockFetch.mockImplementation((url: RequestInfo | URL) => {
        if (typeof url === 'string' && url.includes('/api/bootstrap/status')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ needs_bootstrap: true })
          } as Response)
        }
        return Promise.reject(new Error('Unexpected request'))
      })

      render(React.createElement(App), { wrapper: createWrapper() })

      await waitFor(() => {
        expect(screen.getByTestId('bootstrap-form')).toBeInTheDocument()
      })
    })

    it('should show login form when users exist but no API key', async () => {
      // Mock bootstrap status - users exist
      mockFetch.mockImplementation((url: RequestInfo | URL) => {
        if (typeof url === 'string' && url.includes('/api/bootstrap/status')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ needs_bootstrap: false })
          } as Response)
        }
        return Promise.reject(new Error('Unexpected request'))
      })

      render(React.createElement(App), { wrapper: createWrapper() })

      await waitFor(() => {
        expect(screen.getByTestId('login-form')).toBeInTheDocument()
      })
    })

    it('should show main app when API key exists', async () => {
      // Set API key in localStorage
      localStorage.setItem('apiKey', 'test-api-key')

      // Mock bootstrap status
      mockFetch.mockImplementation((url: RequestInfo | URL) => {
        if (typeof url === 'string' && url.includes('/api/bootstrap/status')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ needs_bootstrap: false })
          } as Response)
        }
        return Promise.reject(new Error('Unexpected request'))
      })

      render(React.createElement(App), { wrapper: createWrapper() })

      await waitFor(() => {
        expect(screen.getByTestId('dashboard-page')).toBeInTheDocument()
      })
    })

    it('should handle login and set API key properly', async () => {
      // Mock bootstrap status - users exist
      // Mock login response
      mockFetch.mockImplementation((url: RequestInfo | URL) => {
        if (typeof url === 'string') {
          if (url.includes('/api/bootstrap/status')) {
            return Promise.resolve({
              ok: true,
              json: () => Promise.resolve({ needs_bootstrap: false })
            } as Response)
          }
          if (url.includes('/api/auth/login')) {
            return Promise.resolve({
              ok: true,
              json: () =>
                Promise.resolve({
                  api_key: 'login-api-key-123',
                  user_id: 'user-123'
                })
            } as Response)
          }
        }
        return Promise.reject(new Error('Unexpected request'))
      })

      render(React.createElement(App), { wrapper: createWrapper() })

      // Wait for login form
      await waitFor(() => {
        expect(screen.getByTestId('login-form')).toBeInTheDocument()
      })

      // Fill and submit login form
      const usernameInput = screen.getByLabelText(/username/i)
      const passwordInput = screen.getByLabelText(/password/i)
      const submitButton = screen.getByRole('button', { name: /login/i })

      fireEvent.change(usernameInput, { target: { value: 'testuser' } })
      fireEvent.change(passwordInput, { target: { value: 'password123' } })
      fireEvent.click(submitButton)

      // Should transition to main app
      await waitFor(() => {
        expect(screen.getByTestId('dashboard-page')).toBeInTheDocument()
      })

      // Verify API key was set
      expect(localStorage.getItem('apiKey')).toBe('login-api-key-123')
    })

    it('should handle logout and show login form without refresh', async () => {
      // Start with API key set
      localStorage.setItem('apiKey', 'test-api-key')

      let bootstrapCallCount = 0

      // Mock bootstrap status - users exist
      mockFetch.mockImplementation((url: RequestInfo | URL) => {
        if (typeof url === 'string' && url.includes('/api/bootstrap/status')) {
          bootstrapCallCount++
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ needs_bootstrap: false })
          } as Response)
        }
        return Promise.reject(new Error('Unexpected request'))
      })

      render(React.createElement(App), { wrapper: createWrapper() })

      // Wait for main app
      await waitFor(() => {
        expect(screen.getByTestId('dashboard-page')).toBeInTheDocument()
      })

      // Click logout
      const logoutButton = screen.getByRole('button', { name: /logout/i })
      fireEvent.click(logoutButton)

      // Should show login form (not bootstrap form)
      await waitFor(() => {
        expect(screen.getByTestId('login-form')).toBeInTheDocument()
      })

      // Verify bootstrap status was re-fetched (should be called again)
      expect(bootstrapCallCount).toBeGreaterThan(1)
    })

    it('should handle complete authentication flow', async () => {
      let bootstrapCallCount = 0

      // Mock all API calls
      mockFetch.mockImplementation((url: RequestInfo | URL) => {
        if (typeof url === 'string') {
          if (url.includes('/api/bootstrap/status')) {
            bootstrapCallCount++
            return Promise.resolve({
              ok: true,
              json: () => Promise.resolve({ needs_bootstrap: false })
            } as Response)
          }
          if (url.includes('/api/auth/login')) {
            return Promise.resolve({
              ok: true,
              json: () =>
                Promise.resolve({
                  api_key: 'flow-api-key-456',
                  user_id: 'user-456'
                })
            } as Response)
          }
        }
        return Promise.reject(new Error('Unexpected request'))
      })

      render(React.createElement(App), { wrapper: createWrapper() })

      // 1. Should show login form initially
      await waitFor(() => {
        expect(screen.getByTestId('login-form')).toBeInTheDocument()
      })

      // 2. Login
      const usernameInput = screen.getByLabelText(/username/i)
      const passwordInput = screen.getByLabelText(/password/i)
      const loginButton = screen.getByRole('button', { name: /login/i })

      fireEvent.change(usernameInput, { target: { value: 'testuser' } })
      fireEvent.change(passwordInput, { target: { value: 'password123' } })
      fireEvent.click(loginButton)

      // 3. Should show main app
      await waitFor(() => {
        expect(screen.getByTestId('dashboard-page')).toBeInTheDocument()
      })

      // 4. Logout
      const logoutButton = screen.getByRole('button', { name: /logout/i })
      fireEvent.click(logoutButton)

      // 5. Should show login form again (not bootstrap)
      await waitFor(() => {
        expect(screen.getByTestId('login-form')).toBeInTheDocument()
      })

      // Verify bootstrap status was called multiple times
      expect(bootstrapCallCount).toBeGreaterThan(1)
    })
  })
})
