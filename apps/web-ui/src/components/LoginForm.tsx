import { useMutation } from '@tanstack/react-query'
import { useState } from 'react'

const API_BASE = (import.meta.env.VITE_API_BASE || 'http://localhost:3000').replace(/\/$/, '')

interface LoginFormProps {
  onApiKeySet: (apiKey: string) => void
}

export function LoginForm({ onApiKeySet }: LoginFormProps) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')

  const loginMutation = useMutation({
    mutationFn: async () => {
      const res = await fetch(`${API_BASE}/api/auth/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username, password })
      })
      if (!res.ok) {
        const errorData = await res.json().catch(() => ({ error: 'Login failed' }))
        throw new Error(errorData.error || 'Login failed')
      }
      return res.json()
    },
    onSuccess: (data) => {
      if (data.api_key) {
        onApiKeySet(data.api_key)
      }
    },
    onError: (error: Error) => {
      setError(error.message)
    }
  })

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    setError('')
    loginMutation.mutate()
  }

  const handleInputChange = () => {
    if (error) setError('')
  }

  return (
    <div className="max-w-md mx-auto p-4 border rounded" data-testid="login-form">
      <h2 className="text-xl font-semibold mb-4">Login</h2>
      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label htmlFor="username" className="block text-sm font-medium">
            Username
          </label>
          <input
            id="username"
            type="text"
            value={username}
            onChange={(e) => {
              setUsername(e.target.value)
              handleInputChange()
            }}
            className="w-full border p-2"
            required
          />
        </div>
        <div>
          <label htmlFor="password" className="block text-sm font-medium">
            Password
          </label>
          <input
            id="password"
            type="password"
            value={password}
            onChange={(e) => {
              setPassword(e.target.value)
              handleInputChange()
            }}
            className="w-full border p-2"
            required
          />
        </div>
        <button
          type="submit"
          disabled={loginMutation.isPending}
          className="w-full bg-blue-500 text-white py-2 rounded disabled:opacity-50"
        >
          {loginMutation.isPending ? 'Logging in...' : 'Login'}
        </button>
      </form>
      {error && <div className="text-red-500 mt-2">{error}</div>}
    </div>
  )
}
