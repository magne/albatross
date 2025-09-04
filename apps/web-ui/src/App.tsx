import { BrowserRouter, Link, Outlet, Route, Routes } from 'react-router'
import { BootstrapAdminForm } from './components/BootstrapAdminForm'
import { AccountPage } from './pages/AccountPage'
import { ApiKeysPage } from './pages/ApiKeysPage'
import { DashboardPage } from './pages/DashboardPage'
import { TenantsPage } from './pages/TenantsPage'
import { UsersPage } from './pages/UsersPage'
import { useApiKey } from './state/ApiKeyContext'

function Layout() {
  const { clear } = useApiKey()
  return (
    <div className="min-h-screen bg-gray-50">
      <nav className="bg-white shadow-sm">
        <div className="max-w-7xl mx-auto px-4">
          <div className="flex justify-between h-16">
            <div className="flex space-x-8">
              <Link to="/" className="flex items-center text-gray-900 hover:text-blue-600">
                Dashboard
              </Link>
              <Link to="/tenants" className="flex items-center text-gray-900 hover:text-blue-600">
                Tenants
              </Link>
              <Link to="/users" className="flex items-center text-gray-900 hover:text-blue-600">
                Users
              </Link>
              <Link to="/apikeys" className="flex items-center text-gray-900 hover:text-blue-600">
                API Keys
              </Link>
              <Link to="/account" className="flex items-center text-gray-900 hover:text-blue-600">
                Account
              </Link>
            </div>
            <button type="button" onClick={clear} className="flex items-center text-gray-900 hover:text-red-600">
              Logout
            </button>
          </div>
        </div>
      </nav>
      <main className="max-w-7xl mx-auto py-6 px-4">
        <Outlet />
      </main>
    </div>
  )
}

function BootstrapWrapper({ onApiKeySet }: { onApiKeySet: (apiKey: string) => void }) {
  return <BootstrapAdminForm onApiKeySet={onApiKeySet} />
}

function App() {
  const { apiKey, setApiKey } = useApiKey()

  if (!apiKey) {
    return <BootstrapWrapper onApiKeySet={setApiKey} />
  }

  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<DashboardPage />} />
          <Route path="tenants" element={<TenantsPage />} />
          <Route path="users" element={<UsersPage />} />
          <Route path="apikeys" element={<ApiKeysPage />} />
          <Route path="account" element={<AccountPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}

export default App
