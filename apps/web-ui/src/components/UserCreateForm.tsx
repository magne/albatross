import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useApi } from '../api/client'
import { useTenants } from '../hooks/useTenants'
import { useApiKey } from '../state/ApiKeyContext'

const ROLES = [
  { value: 1, label: 'ROLE_PLATFORM_ADMIN' },
  { value: 2, label: 'ROLE_TENANT_ADMIN' },
  { value: 3, label: 'ROLE_PILOT' }
]

export function UserCreateForm() {
  const { role } = useApiKey()
  const api = useApi()
  const queryClient = useQueryClient()
  const { data: tenants } = useTenants()
  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [selectedRole, setSelectedRole] = useState(3) // Default to Pilot
  const [selectedTenantId, setSelectedTenantId] = useState('')

  const createMutation = useMutation({
    mutationFn: () => api.createUser(username, email, password, selectedRole, selectedTenantId || undefined),
    onSuccess: () => {
      setUsername('')
      setEmail('')
      setPassword('')
      setSelectedRole(3)
      setSelectedTenantId('')
      queryClient.invalidateQueries({ queryKey: ['users'] })
    }
  })

  const isPlatformAdmin = role === 'ROLE_PLATFORM_ADMIN'
  const isTenantAdmin = role === 'ROLE_TENANT_ADMIN'

  const availableRoles = ROLES.filter((r) => {
    if (isPlatformAdmin) return true
    if (isTenantAdmin) return r.value !== 1 // Cannot create PlatformAdmin
    return false
  })

  const needsTenant = selectedRole !== 1 // PlatformAdmin doesn't need tenant

  return (
    <div>
      <h3 className="text-lg font-semibold mb-4">Create User</h3>
      <div className="mb-4 space-y-2">
        <input
          type="text"
          placeholder="Username"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          className="border p-2 w-full"
        />
        <input
          type="email"
          placeholder="Email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          className="border p-2 w-full"
        />
        <input
          type="password"
          placeholder="Password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          className="border p-2 w-full"
        />
        <select
          value={selectedRole}
          onChange={(e) => setSelectedRole(Number(e.target.value))}
          className="border p-2 w-full"
        >
          {availableRoles.map((r) => (
            <option key={r.value} value={r.value}>
              {r.label}
            </option>
          ))}
        </select>
        {needsTenant && (
          <select
            value={selectedTenantId}
            onChange={(e) => setSelectedTenantId(e.target.value)}
            className="border p-2 w-full"
          >
            <option value="">Select Tenant</option>
            {tenants?.data.map((t) => (
              <option key={t.tenant_id} value={t.tenant_id}>
                {t.name}
              </option>
            ))}
          </select>
        )}
        <button
          type="button"
          onClick={() => createMutation.mutate()}
          disabled={!username || !email || !password || (needsTenant && !selectedTenantId) || createMutation.isPending}
          className="bg-blue-500 text-white px-4 py-2 rounded w-full"
        >
          Create User
        </button>
      </div>
      {createMutation.isError && <div className="text-red-500">Error creating user</div>}
    </div>
  )
}
