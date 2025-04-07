// Declare command handler modules
pub mod change_password; // Added
pub mod create_tenant;
pub mod generate_api_key; // Added
pub mod register_user;
// Add other command handler modules here later:
// pub mod login_user; // Login might be handled differently

// Optional: Re-export handlers or command types if needed
// pub use change_password::ChangePasswordHandler;
// pub use register_user::RegisterUserHandler;
// pub use create_tenant::CreateTenantHandler;
