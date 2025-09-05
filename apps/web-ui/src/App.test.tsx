import { render, screen, waitFor } from '@testing-library/react'
import React from 'react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import '@testing-library/jest-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import App from './App'
import { ApiKeyProvider } from './state/ApiKeyContext'

// Mock the API client
const mockCheckBootstrapStatus = vi.fn()
vi.mock('./api/client', () => ({
  useApi: () => ({
    checkBootstrapStatus: mockCheckBootstrapStatus
  })
}))

// Mock components
vi.mock('./components/BootstrapAdminForm', () => ({
  BootstrapAdminForm: ({ onApiKeySet }: { onApiKeySet: (key: string) => void }) => (
    <div data-testid="bootstrap-form">
      <button type="button" onClick={() => onApiKeySet('test-key')}>
        Bootstrap
      </button>
    </div>
  )
}))

vi.mock('./components/LoginForm', () => ({
  LoginForm: ({ onApiKeySet }: { onApiKeySet: (key: string) => void }) => (
    <div data-testid="login-form">
      <button type="button" onClick={() => onApiKeySet('test-key')}>
        Login
      </button>
    </div>
  )
}))

vi.mock('./pages/DashboardPage', () => ({
  DashboardPage: () => <div data-testid="dashboard-page">Dashboard</div>
}))

vi.mock('./pages/TenantsPage', () => ({
  TenantsPage: () => <div data-testid="tenants-page">Tenants</div>
}))

vi.mock('./pages/UsersPage', () => ({
  UsersPage: () => <div data-testid="users-page">Users</div>
}))

vi.mock('./pages/ApiKeysPage', () => ({
  ApiKeysPage: () => <div data-testid="apikeys-page">API Keys</div>
}))

vi.mock('./pages/AccountPage', () => ({
  AccountPage: () => <div data-testid="account-page">Account</div>
}))

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

describe('App', () => {
  beforeEach(() => {
    mockCheckBootstrapStatus.mockClear()
  })

  it('shows loading state while checking bootstrap status', async () => {
    mockCheckBootstrapStatus.mockResolvedValue({ needs_bootstrap: true })

    render(React.createElement(App), { wrapper: createWrapper() })

    expect(screen.getByText('Loading...')).toBeInTheDocument()
  })

  it('shows bootstrap form when needs_bootstrap is true and no API key', async () => {
    mockCheckBootstrapStatus.mockResolvedValue({ needs_bootstrap: true })

    render(React.createElement(App), { wrapper: createWrapper() })

    await waitFor(() => {
      expect(screen.getByTestId('bootstrap-form')).toBeInTheDocument()
    })
  })

  it('shows login form when needs_bootstrap is false and no API key', async () => {
    mockCheckBootstrapStatus.mockResolvedValue({ needs_bootstrap: false })

    render(React.createElement(App), { wrapper: createWrapper() })

    await waitFor(() => {
      expect(screen.getByTestId('login-form')).toBeInTheDocument()
    })
  })

  it('shows error message when bootstrap status check fails', async () => {
    mockCheckBootstrapStatus.mockRejectedValue(new Error('Network error'))

    render(React.createElement(App), { wrapper: createWrapper() })

    await waitFor(() => {
      expect(screen.getByText('Failed to load application. Please refresh the page.')).toBeInTheDocument()
    })
  })

  it('shows main app when API key exists', async () => {
    // Set up API key in localStorage
    localStorage.setItem('apiKey', 'test-key')
    mockCheckBootstrapStatus.mockResolvedValue({ needs_bootstrap: false })

    render(React.createElement(App), { wrapper: createWrapper() })

    await waitFor(() => {
      expect(screen.getByTestId('dashboard-page')).toBeInTheDocument()
    })

    // Clean up
    localStorage.removeItem('apiKey')
  })
})
