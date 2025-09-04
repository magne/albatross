import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useApi } from '../api/client'

export function TenantCreateForm() {
  const api = useApi()
  const queryClient = useQueryClient()
  const [name, setName] = useState('')

  const createMutation = useMutation({
    mutationFn: () => api.createTenant(name),
    onSuccess: () => {
      setName('')
      queryClient.invalidateQueries({ queryKey: ['tenants'] })
    }
  })

  return (
    <div>
      <h3 className="text-lg font-semibold mb-4">Create Tenant</h3>
      <div className="mb-4">
        <input
          type="text"
          placeholder="Tenant name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          className="border p-2 mr-2"
        />
        <button
          type="button"
          onClick={() => createMutation.mutate()}
          disabled={!name || createMutation.isPending}
          className="bg-blue-500 text-white px-4 py-2 rounded"
        >
          Create
        </button>
      </div>
      {createMutation.isError && <div className="text-red-500">Error creating tenant</div>}
    </div>
  )
}
