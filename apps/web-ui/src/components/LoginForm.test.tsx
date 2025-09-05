import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import type React from 'react'
import '@testing-library/jest-dom'
import { beforeEach, describe, expect, it, type MockedFunction, vi } from 'vitest'
import { ApiKeyProvider } from '../state/ApiKeyContext'
import { LoginForm } from './LoginForm'

// Mock fetch globally with proper typing
const mockFetch = vi.fn() as MockedFunction<typeof fetch>
globalThis.fetch = mockFetch

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false
      }
    }
  })
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      <ApiKeyProvider>{children}</ApiKeyProvider>
    </QueryClientProvider>
  )
}

describe('LoginForm', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should render login form with username and password fields', () => {
    render(<LoginForm onApiKeySet={() => {}} />, {
      wrapper: createWrapper()
    })

    expect(screen.getByLabelText(/username/i)).toBeInTheDocument()
    expect(screen.getByLabelText(/password/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /login/i })).toBeInTheDocument()
  })

  it('should show loading state during submission', async () => {
    // Mock successful login
    const mockApiKey = 'test-api-key-123'
    const mockResponse = { api_key: mockApiKey, user_id: 'user-123' }
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(mockResponse)
    } as Response)

    render(<LoginForm onApiKeySet={() => {}} />, {
      wrapper: createWrapper()
    })

    const usernameInput = screen.getByLabelText(/username/i)
    const passwordInput = screen.getByLabelText(/password/i)
    const submitButton = screen.getByRole('button', { name: /login/i })

    fireEvent.change(usernameInput, { target: { value: 'testuser' } })
    fireEvent.change(passwordInput, { target: { value: 'password123' } })
    fireEvent.click(submitButton)

    // Button should show loading state
    expect(screen.getByRole('button', { name: /logging in/i })).toBeInTheDocument()
    expect(submitButton).toBeDisabled()

    await waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/auth/login'),
        expect.objectContaining({
          method: 'POST',
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({
            username: 'testuser',
            password: 'password123'
          })
        })
      )
    })
  })

  it('should call onApiKeySet with returned API key on successful login', async () => {
    const mockOnApiKeySet = vi.fn()
    const mockApiKey = 'test-api-key-123'
    const mockResponse = { api_key: mockApiKey, user_id: 'user-123' }

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(mockResponse)
    } as Response)

    render(<LoginForm onApiKeySet={mockOnApiKeySet} />, {
      wrapper: createWrapper()
    })

    const usernameInput = screen.getByLabelText(/username/i)
    const passwordInput = screen.getByLabelText(/password/i)
    const submitButton = screen.getByRole('button', { name: /login/i })

    fireEvent.change(usernameInput, { target: { value: 'testuser' } })
    fireEvent.change(passwordInput, { target: { value: 'password123' } })
    fireEvent.click(submitButton)

    await waitFor(() => {
      expect(mockOnApiKeySet).toHaveBeenCalledWith(mockApiKey)
    })
  })

  it('should display error message on login failure', async () => {
    const mockError = { error: 'Invalid credentials' }
    mockFetch.mockResolvedValueOnce({
      ok: false,
      json: () => Promise.resolve(mockError)
    } as Response)

    render(<LoginForm onApiKeySet={() => {}} />, {
      wrapper: createWrapper()
    })

    const usernameInput = screen.getByLabelText(/username/i)
    const passwordInput = screen.getByLabelText(/password/i)
    const submitButton = screen.getByRole('button', { name: /login/i })

    fireEvent.change(usernameInput, { target: { value: 'testuser' } })
    fireEvent.change(passwordInput, { target: { value: 'wrongpassword' } })
    fireEvent.click(submitButton)

    await waitFor(() => {
      expect(screen.getByText('Invalid credentials')).toBeInTheDocument()
    })

    // Button should be enabled again after error
    expect(screen.getByRole('button', { name: /login/i })).not.toBeDisabled()
  })

  it('should handle network errors gracefully', async () => {
    mockFetch.mockRejectedValueOnce(new Error('Network error'))

    render(<LoginForm onApiKeySet={() => {}} />, {
      wrapper: createWrapper()
    })

    const usernameInput = screen.getByLabelText(/username/i)
    const passwordInput = screen.getByLabelText(/password/i)
    const submitButton = screen.getByRole('button', { name: /login/i })

    fireEvent.change(usernameInput, { target: { value: 'testuser' } })
    fireEvent.change(passwordInput, { target: { value: 'password123' } })
    fireEvent.click(submitButton)

    await waitFor(() => {
      expect(screen.getByText('Network error')).toBeInTheDocument()
    })
  })

  it('should require both username and password', () => {
    render(<LoginForm onApiKeySet={() => {}} />, {
      wrapper: createWrapper()
    })

    const submitButton = screen.getByRole('button', { name: /login/i })

    // Try to submit without filling fields
    fireEvent.click(submitButton)

    // Form should prevent submission (HTML5 validation)
    expect(globalThis.fetch).not.toHaveBeenCalled()
  })

  it('should clear error message when user starts typing', async () => {
    const mockError = { error: 'Invalid credentials' }
    mockFetch.mockResolvedValueOnce({
      ok: false,
      json: () => Promise.resolve(mockError)
    } as Response)

    render(<LoginForm onApiKeySet={() => {}} />, {
      wrapper: createWrapper()
    })

    const usernameInput = screen.getByLabelText(/username/i)
    const passwordInput = screen.getByLabelText(/password/i)
    const submitButton = screen.getByRole('button', { name: /login/i })

    // Trigger error first
    fireEvent.change(usernameInput, { target: { value: 'testuser' } })
    fireEvent.change(passwordInput, { target: { value: 'wrongpassword' } })
    fireEvent.click(submitButton)

    await waitFor(() => {
      expect(screen.getByText('Invalid credentials')).toBeInTheDocument()
    })

    // Start typing in username field
    fireEvent.change(usernameInput, { target: { value: 'newuser' } })

    // Error should be cleared
    expect(screen.queryByText('Invalid credentials')).not.toBeInTheDocument()
  })
})
