import { useUsers } from '../hooks/useUsers'

export function UserList() {
  const { data, isLoading, error } = useUsers()

  if (isLoading) return <div>Loading users...</div>
  if (error) return <div>Error loading users</div>
  if (!data?.data) return <div>No users found</div>

  return (
    <div>
      <h2 className="text-xl font-semibold mb-4">Users</h2>
      <ul className="space-y-2">
        {data.data.map((user) => (
          <li key={user.user_id} className="p-4 border rounded">
            <div className="font-medium">{user.username}</div>
            <div className="text-sm text-gray-600">Email: {user.email}</div>
            <div className="text-sm text-gray-600">Role: {user.role}</div>
          </li>
        ))}
      </ul>
    </div>
  )
}
