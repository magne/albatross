import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { type ApiKeyDto, useApi } from '../api/client'
import { useUserApiKeys } from '../hooks/useUserApiKeys'
import { useApiKey } from '../state/ApiKeyContext'

export function ApiKeyPanel() {
  const { userId } = useApiKey()
  const { data: apiKeys, isLoading } = useUserApiKeys()
  const api = useApi()
  const queryClient = useQueryClient()
  const [keyName, setKeyName] = useState('')

  const generateMutation = useMutation({
    mutationFn: () => (userId ? api.generateApiKey(userId, keyName) : Promise.reject(new Error('No user ID'))),
    onSuccess: (data) => {
      alert(`New API Key: ${data.api_key} (copy it now, it won't be shown again)`)
      setKeyName('')
      if (userId) {
        queryClient.invalidateQueries({ queryKey: ['user_api_keys', userId] })
      }
    }
  })

  const revokeMutation = useMutation({
    mutationFn: (keyId: string) => (userId ? api.revokeApiKey(userId, keyId) : Promise.reject(new Error('No user ID'))),
    onSuccess: () => {
      if (userId) {
        queryClient.invalidateQueries({ queryKey: ['user_api_keys', userId] })
      }
    }
  })

  if (!userId) return <div>Please set API key first</div>
  if (isLoading) return <div>Loading API keys...</div>

  return (
    <div>
      <h2 className="text-xl font-semibold mb-4">API Keys</h2>
      <div className="mb-4">
        <input
          type="text"
          placeholder="Key name"
          value={keyName}
          onChange={(e) => setKeyName(e.target.value)}
          className="border p-2 mr-2"
        />
        <button
          type="button"
          onClick={() => generateMutation.mutate()}
          disabled={!keyName || generateMutation.isPending}
          className="bg-blue-500 text-white px-4 py-2 rounded"
        >
          Generate Key
        </button>
      </div>
      <ul className="space-y-2">
        {(apiKeys || []).map((key: ApiKeyDto) => (
          <li key={key.key_id} className="p-4 border rounded flex justify-between">
            <div>
              <div className="font-medium">{key.key_name}</div>
              <div className="text-sm text-gray-600">ID: {key.key_id}</div>
            </div>
            <button
              type="button"
              onClick={() => revokeMutation.mutate(key.key_id)}
              disabled={revokeMutation.isPending}
              className="bg-red-500 text-white px-3 py-1 rounded"
            >
              Revoke
            </button>
          </li>
        ))}
      </ul>
    </div>
  )
}
