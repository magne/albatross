import { TenantCreateForm } from '../components/TenantCreateForm'
import { TenantList } from '../components/TenantList'
import { useApiKey } from '../state/ApiKeyContext'

export function TenantsPage() {
  const { role } = useApiKey()

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6">Tenants</h1>
      {(role === 'PlatformAdmin' || role === 'ROLE_PLATFORM_ADMIN') && (
        <div className="mb-6">
          <TenantCreateForm />
        </div>
      )}
      <TenantList />
    </div>
  )
}
