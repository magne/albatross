use chrono::{DateTime, Utc};
use sqlx::FromRow;

// Represents the 'users' read model table
#[derive(FromRow, Debug)]
pub struct UserDetails {
    pub user_id: String,
    pub tenant_id: Option<String>,
    pub username: String,
    pub email: String,
    pub role: String, // Stored as string in read model
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Represents the 'tenants' read model table
#[derive(FromRow, Debug)]
pub struct TenantDetails {
    pub tenant_id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Represents the 'user_api_keys' read model table
#[derive(FromRow, Debug)]
pub struct ApiKeyDetails {
    pub key_id: String,
    pub user_id: String,
    pub tenant_id: Option<String>,
    pub key_name: String,
    pub api_key_hash: String,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}
