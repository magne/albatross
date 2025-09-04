import { useTenants } from '../hooks/useTenants'

export function TenantList() {
  const { data, isLoading, error } = useTenants()

  if (isLoading) return <div>Loading tenants...</div>
  if (error) return <div>Error loading tenants</div>
  if (!data?.data) return <div>No tenants found</div>

  return (
    <div>
      <h2 className="text-xl font-semibold mb-4">Tenants</h2>
      <ul className="space-y-2">
        {data.data.map((tenant) => (
          <li key={tenant.tenant_id} className="p-4 border rounded">
            <div className="font-medium">{tenant.name}</div>
            <div className="text-sm text-gray-600">ID: {tenant.tenant_id}</div>
          </li>
        ))}
      </ul>
    </div>
  )
}
