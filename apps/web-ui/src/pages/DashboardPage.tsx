import { useTenants } from '../hooks/useTenants'
import { useUserSelf } from '../hooks/useUsers'
import { useRealtime } from '../realtime/useRealtime'

export function DashboardPage() {
  const { data: user } = useUserSelf()
  const { data: tenants } = useTenants()
  const { status: wsStatus, eventsProcessed } = useRealtime()

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6">Dashboard</h1>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <div className="p-4 border rounded">
          <h2 className="text-lg font-semibold">Welcome</h2>
          <p>{user?.username || 'Loading...'}</p>
          <p className="text-sm text-gray-600">Role: {user?.role}</p>
        </div>
        <div className="p-4 border rounded">
          <h2 className="text-lg font-semibold">Tenants</h2>
          <p>{tenants?.data?.length || 0} total</p>
        </div>
        <div className="p-4 border rounded">
          <h2 className="text-lg font-semibold">Real-time Status</h2>
          <p>Status: {wsStatus}</p>
          <p className="text-sm text-gray-600">Events: {eventsProcessed}</p>
        </div>
      </div>
    </div>
  )
}
