import { useMutation } from '@tanstack/react-query'
import { useState } from 'react'
import { useApi } from '../api/client'
import { useApiKey } from '../state/ApiKeyContext'

export function ChangePasswordForm() {
  const { userId } = useApiKey()
  const api = useApi()
  const [oldPassword, setOldPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')

  const changePasswordMutation = useMutation({
    mutationFn: () => {
      if (!userId) throw new Error('No user ID')
      // Note: The backend expects the new password hash, but for now we'll send the plain password
      // In a real implementation, you'd hash the password on the client side
      return api.changePassword(userId, oldPassword, newPassword)
    },
    onSuccess: () => {
      setOldPassword('')
      setNewPassword('')
      setConfirmPassword('')
      alert('Password changed successfully')
    }
  })

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (newPassword !== confirmPassword) {
      alert('New passwords do not match')
      return
    }
    if (newPassword.length < 6) {
      alert('New password must be at least 6 characters long')
      return
    }
    changePasswordMutation.mutate()
  }

  return (
    <div>
      <h3 className="text-lg font-semibold mb-4">Change Password</h3>
      <form onSubmit={handleSubmit} className="space-y-2">
        <input
          type="password"
          placeholder="Current password"
          value={oldPassword}
          onChange={(e) => setOldPassword(e.target.value)}
          className="border p-2 w-full"
        />
        <input
          type="password"
          placeholder="New password"
          value={newPassword}
          onChange={(e) => setNewPassword(e.target.value)}
          className="border p-2 w-full"
        />
        <input
          type="password"
          placeholder="Confirm new password"
          value={confirmPassword}
          onChange={(e) => setConfirmPassword(e.target.value)}
          className="border p-2 w-full"
        />
        <button type="submit" className="bg-blue-500 text-white px-4 py-2 rounded w-full">
          Change Password
        </button>
      </form>
    </div>
  )
}
