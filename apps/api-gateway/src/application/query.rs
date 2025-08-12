use axum::{
    extract::{Query, State, Extension},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{debug, warn};

use crate::AppState;
use super::middleware::AuthenticatedUser;
use super::authz::{parse_role, AuthRole};

const TTL_LIST_SECONDS: u64 = 45;
const TTL_SELF_SECONDS: u64 = 60;
const MAX_LIMIT: u32 = 200;
const DEFAULT_LIMIT: u32 = 50;

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
struct UserRow {
    user_id: String,
    tenant_id: Option<String>,
    username: String,
    email: String,
    role: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
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
    app.pg_pool.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)
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
