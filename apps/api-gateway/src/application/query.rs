use axum::{
    extract::{Query, State, Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{debug, warn};

use crate::AppState;
use super::middleware::AuthenticatedUser;
use super::authz::{parse_role, AuthRole, authorize, Requirement};

const TTL_LIST_SECONDS: u64 = 45;
const TTL_SELF_SECONDS: u64 = 60;
const MAX_LIMIT: u32 = 200;
const DEFAULT_LIMIT: u32 = 50;
const TTL_API_KEYS_SECONDS: u64 = 30;

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(sqlx::FromRow, Serialize)]
struct TenantRow {
    tenant_id: String,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct UserRow {
    user_id: String,
    tenant_id: Option<String>,
    username: String,
    email: String,
    role: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow, Serialize)]
struct ApiKeyRow {
    key_id: String,
    key_name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn normalize_pagination(p: &Pagination) -> (u32, u32) {
    let mut limit = p.limit.unwrap_or(DEFAULT_LIMIT);
    if limit == 0 {
        limit = DEFAULT_LIMIT;
    }
    if limit > MAX_LIMIT {
        limit = MAX_LIMIT;
    }
    let offset = p.offset.unwrap_or(0);
    (limit, offset)
}

// GET /api/users/{user_id}/apikeys (list)
pub async fn handle_list_user_api_keys(
    State(app_state): State<AppState>,
    Extension(ctx): Extension<AuthenticatedUser>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let pool = ensure_pool(&app_state).await?;
    // Authorize (self or tenant admin for same tenant)
    // Load target user's tenant_id for cross-user access decisions
    // Runtime (non-macro) query to avoid SQLX_OFFLINE preparation requirement
    let target_user_row = sqlx::query!(
        "SELECT tenant_id::text, username FROM users WHERE user_id = $1",
        user_id
    )
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let target_user_row = match target_user_row {
        Some(row) => row,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let role = parse_role(&ctx.role).ok_or(StatusCode::FORBIDDEN)?;
    authorize(
        &ctx.user_id,
        &ctx.tenant_id,
        role,
        Requirement::SelfOrTenantAdmin {
            target_user_id: user_id.clone(),
            target_tenant_id: target_user_row.tenant_id,
        },
    )
    .map_err(|_| StatusCode::FORBIDDEN)?;

    let cache_key = format!("q:v1:user_api_keys:{user_id}");
    if let Ok(Some(bytes)) = app_state.cache.get(&cache_key).await {
        if let Ok(resp) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            debug!("cache hit user_api_keys key={}", cache_key);
            return Ok((StatusCode::OK, Json(resp)));
        }
    }

    let rows: Vec<ApiKeyRow> = sqlx::query_as::<_, ApiKeyRow>(
        r#"
        SELECT key_id, key_name, created_at, revoked_at, last_used_at
        FROM user_api_keys
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(&user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        warn!("DB error listing user api keys: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response = serde_json::json!({
        "data": rows,
        "user_id": user_id,
        "pagination": {
            "returned": rows.len()
        }
    });

    if let Ok(bytes) = serde_json::to_vec(&response) {
        let _ = app_state.cache.set(&cache_key, &bytes, Some(TTL_API_KEYS_SECONDS));
    }

    Ok((StatusCode::OK, Json(response)))
}

fn cache_key_tenants(role: AuthRole, tenant_id: Option<&str>) -> String {
    match role {
        AuthRole::PlatformAdmin => "q:v1:tenants:all".to_string(),
        AuthRole::TenantAdmin | AuthRole::Pilot => {
            if let Some(tid) = tenant_id {
                format!("q:v1:tenants:tenant:{tid}")
            } else {
                // Should not generally happen; fallback distinct key
                "q:v1:tenants:none".to_string()
            }
        }
    }
}

fn cache_key_users(role: AuthRole, tenant_id: Option<&str>, user_id: &str, limit: u32, offset: u32) -> String {
    match role {
        AuthRole::PlatformAdmin => format!("q:v1:users:all:limit:{limit}:offset:{offset}"),
        AuthRole::TenantAdmin => {
            if let Some(tid) = tenant_id {
                format!("q:v1:users:tenant:{tid}:limit:{limit}:offset:{offset}")
            } else {
                // Fallback
                format!("q:v1:users:tenant:none:limit:{limit}:offset:{offset}")
            }
        }
        AuthRole::Pilot => format!("q:v1:users:self:{user_id}"),
    }
}

async fn ensure_pool(app: &AppState) -> Result<&PgPool, StatusCode> {
    app.pg_pool.as_ref().ok_or_else(|| {
        warn!("PostgreSQL pool not available - database queries disabled");
        StatusCode::SERVICE_UNAVAILABLE
    })
}

// GET /api/tenants
pub async fn handle_list_tenants(
    State(app_state): State<AppState>,
    Extension(ctx): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, StatusCode> {
    let role = parse_role(&ctx.role).ok_or(StatusCode::FORBIDDEN)?;
    let pool = ensure_pool(&app_state).await?;

    let cache_key = cache_key_tenants(role, ctx.tenant_id.as_deref());

    if let Ok(Some(bytes)) = app_state.cache.get(&cache_key).await {
        if let Ok(resp) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            debug!("cache hit tenants key={}", cache_key);
            return Ok((StatusCode::OK, Json(resp)));
        }
    }

    let rows: Vec<TenantRow> = match role {
        AuthRole::PlatformAdmin => {
            sqlx::query_as::<_, TenantRow>(
                r#"
                SELECT tenant_id, name, created_at, updated_at
                FROM tenants
                ORDER BY created_at DESC
                "#,
            )
            .fetch_all(pool)
            .await
            .map_err(|e| {
                warn!("DB error listing tenants: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
        }
        AuthRole::TenantAdmin | AuthRole::Pilot => {
            let tid = ctx.tenant_id.as_ref().ok_or(StatusCode::FORBIDDEN)?;
            sqlx::query_as::<_, TenantRow>(
                r#"
                SELECT tenant_id, name, created_at, updated_at
                FROM tenants
                WHERE tenant_id = $1
                "#,
            )
            .bind(tid)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                warn!("DB error listing scoped tenant: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
        }
    };

    let response = serde_json::json!({
        "data": rows,
        "pagination": {
            "limit": rows.len(),
            "offset": 0,
            "returned": rows.len()
        }
    });

    let ttl = if matches!(role, AuthRole::PlatformAdmin) { TTL_LIST_SECONDS } else { TTL_LIST_SECONDS };
    if let Ok(bytes) = serde_json::to_vec(&response) {
        let _ = app_state.cache.set(&cache_key, &bytes, Some(ttl));
    }

    Ok((StatusCode::OK, Json(response)))
}

// GET /api/users
pub async fn handle_list_users(
    State(app_state): State<AppState>,
    Extension(ctx): Extension<AuthenticatedUser>,
    Query(p): Query<Pagination>,
) -> Result<impl IntoResponse, StatusCode> {
    let role = parse_role(&ctx.role).ok_or(StatusCode::FORBIDDEN)?;
    let pool = ensure_pool(&app_state).await?;

    let (limit, offset) = normalize_pagination(&p);
    let cache_key = cache_key_users(role, ctx.tenant_id.as_deref(), &ctx.user_id, limit, offset);

    if let Ok(Some(bytes)) = app_state.cache.get(&cache_key).await {
        if let Ok(resp) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            debug!("cache hit users key={}", cache_key);
            return Ok((StatusCode::OK, Json(resp)));
        }
    }

    let rows: Vec<UserRow> = match role {
        AuthRole::PlatformAdmin => {
            sqlx::query_as::<_, UserRow>(
                r#"
                SELECT user_id, tenant_id, username, email, role, created_at, updated_at
                FROM users
                ORDER BY created_at DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                warn!("DB error listing users (PA): {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
        }
        AuthRole::TenantAdmin => {
            let tid = ctx.tenant_id.as_ref().ok_or(StatusCode::FORBIDDEN)?;
            sqlx::query_as::<_, UserRow>(
                r#"
                SELECT user_id, tenant_id, username, email, role, created_at, updated_at
                FROM users
                WHERE tenant_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(tid)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                warn!("DB error listing users (TA): {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
        }
        AuthRole::Pilot => {
            // Self only
            sqlx::query_as::<_, UserRow>(
                r#"
                SELECT user_id, tenant_id, username, email, role, created_at, updated_at
                FROM users
                WHERE user_id = $1
                "#,
            )
            .bind(&ctx.user_id)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                warn!("DB error listing self user (Pilot): {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
        }
    };

    let response = serde_json::json!({
        "data": rows,
        "pagination": {
            "limit": limit,
            "offset": offset,
            "returned": rows.len()
        }
    });

    let ttl = if matches!(role, AuthRole::Pilot) { TTL_SELF_SECONDS } else { TTL_LIST_SECONDS };
    if let Ok(bytes) = serde_json::to_vec(&response) {
        let _ = app_state.cache.set(&cache_key, &bytes, Some(ttl));
    }

    Ok((StatusCode::OK, Json(response)))
}
