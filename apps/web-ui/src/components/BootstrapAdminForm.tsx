import { useMutation } from '@tanstack/react-query'
import { useState } from 'react'

const API_BASE = (import.meta.env.VITE_API_BASE || 'http://localhost:3000').replace(/\/$/, '')

interface BootstrapAdminFormProps {
  onApiKeySet: (apiKey: string) => void
}

export function BootstrapAdminForm({ onApiKeySet }: BootstrapAdminFormProps) {
  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')

  const bootstrapMutation = useMutation({
    mutationFn: async () => {
      const res = await fetch(`${API_BASE}/api/users`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          username,
          email,
          password_plaintext: password,
          initial_role: 1, // PlatformAdmin role
          tenant_id: null
        })
      })
      if (!res.ok) throw new Error('Bootstrap failed')
      return res.json()
    },
    onSuccess: (data) => {
      // For bootstrap, we need to generate an API key after user creation
      // The backend should return the user_id, then we can generate a key
      if (data.user_id) {
        // Generate API key for the newly created user
        fetch(`${API_BASE}/api/users/${data.user_id}/apikeys`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ key_name: 'bootstrap-key' })
        })
          .then((res) => res.json())
          .then((keyData) => {
            if (keyData.api_key) {
              onApiKeySet(keyData.api_key)
              alert('Bootstrap successful! API key set.')
            }
          })
          .catch((err) => {
            console.error('Failed to generate API key:', err)
            alert('User created but failed to generate API key. Please try generating one manually.')
          })
      }
    }
  })

  return (
    <div className="max-w-md mx-auto p-4 border rounded" data-testid="bootstrap-form">
      <h2 className="text-xl font-semibold mb-4">Bootstrap First Admin</h2>
      <form
        onSubmit={(e) => {
          e.preventDefault()
          bootstrapMutation.mutate()
        }}
        className="space-y-4"
      >
        <div>
          <label htmlFor="username" className="block text-sm font-medium">
            Username
          </label>
          <input
            id="username"
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            className="w-full border p-2"
            required
          />
        </div>
        <div>
          <label htmlFor="email" className="block text-sm font-medium">
            Email
          </label>
          <input
            id="email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
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
            onChange={(e) => setPassword(e.target.value)}
            className="w-full border p-2"
            required
          />
        </div>
        <button
          type="submit"
          disabled={bootstrapMutation.isPending}
          className="w-full bg-green-500 text-white py-2 rounded"
        >
          Bootstrap Admin
        </button>
      </form>
      {bootstrapMutation.error && <div className="text-red-500 mt-2">{bootstrapMutation.error.message}</div>}
    </div>
  )
}
