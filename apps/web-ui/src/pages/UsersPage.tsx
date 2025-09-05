import { UserCreateForm } from '../components/UserCreateForm'
import { UserList } from '../components/UserList'
import { useApiKey } from '../state/ApiKeyContext'

export function UsersPage() {
  const { role } = useApiKey()

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6">Users</h1>
      {(role === 'PlatformAdmin' || role === 'TenantAdmin' || role === 'ROLE_PLATFORM_ADMIN' || role === 'ROLE_TENANT_ADMIN') && (
        <div className="mb-6">
          <UserCreateForm />
        </div>
      )}
      <UserList />
    </div>
  )
}
