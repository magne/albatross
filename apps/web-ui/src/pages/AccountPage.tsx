import { ChangePasswordForm } from '../components/ChangePasswordForm'

export function AccountPage() {
  return (
    <div>
      <h1 className="text-2xl font-bold mb-6">Account Settings</h1>
      <div className="max-w-md">
        <ChangePasswordForm />
      </div>
    </div>
  )
}
