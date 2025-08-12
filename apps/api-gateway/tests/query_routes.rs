use api_gateway::{create_app, AppState};
use axum::http::{HeaderValue, StatusCode};
use axum_test::TestServer;
use core_lib::{
    adapters::{
        in_memory_cache::InMemoryCache, in_memory_event_bus::InMemoryEventBus,
        in_memory_repository::InMemoryEventRepository,
    },
    Cache, EventPublisher, Repository,
};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Arc;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

/// Helper to spin up a Postgres test container and return a PgPool.
async fn setup_pg() -> PgPool {
    // Start postgres container (default: user=postgres, password=postgres, db=postgres)
    // Start container (unwrap to get container) and leak it so it lives for test duration
    let container = Postgres::default().start().await.unwrap();
    let port: u16 = container
        .get_host_port_ipv4(5432)
        .await
        .expect("retrieve mapped postgres port");
    // Construct connection string assuming default credentials from the Postgres module
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    Box::leak(Box::new(container)); // Leak so container lives for test duration
    // Wait briefly for readiness (simple approach; sqlx connect will also retry internally)
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("connect postgres");
    // Apply minimal schema (split into individual statements; PostgreSQL disallows multi-stmt in prepared)
    sqlx::query("CREATE TABLE tenants (tenant_id VARCHAR(36) PRIMARY KEY, name VARCHAR(255) NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())")
        .execute(&pool).await.expect("create tenants");
    sqlx::query("CREATE TABLE users (user_id VARCHAR(36) PRIMARY KEY, tenant_id VARCHAR(36) NULL, username VARCHAR(100) NOT NULL UNIQUE, email VARCHAR(255) NOT NULL UNIQUE, role VARCHAR(50) NOT NULL, password_hash VARCHAR(255) NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())")
        .execute(&pool).await.expect("create users");
    sqlx::query("CREATE INDEX idx_users_tenant_id ON users(tenant_id)")
        .execute(&pool).await.expect("index tenant_id");
    sqlx::query("CREATE INDEX idx_users_username ON users(username)")
        .execute(&pool).await.expect("index username");
    sqlx::query("CREATE INDEX idx_users_email ON users(email)")
        .execute(&pool).await.expect("index email");
    pool
}

/// Seed baseline tenants and users for query tests.
async fn seed_data(pool: &PgPool) {
    // Tenants
    sqlx::query(r#"INSERT INTO tenants (tenant_id, name) VALUES
        ('tenant-a','Alpha VA'),
        ('tenant-b','Bravo VA')"#)
        .execute(pool)
        .await
        .expect("insert tenants");

    // Users: PlatformAdmin (no tenant), TenantAdmin A, Pilot A, TenantAdmin B
    sqlx::query(r#"INSERT INTO users (user_id, tenant_id, username, email, role, password_hash)
        VALUES
        ('user-pa', NULL, 'platform', 'platform@example.com', 'PlatformAdmin', 'hash'),
        ('user-ta1','tenant-a','ta1','ta1@example.com','TenantAdmin','hash'),
        ('user-pi1','tenant-a','pilot1','pilot1@example.com','Pilot','hash'),
        ('user-ta2','tenant-b','ta2','ta2@example.com','TenantAdmin','hash')"#)
        .execute(pool)
        .await
        .expect("insert users");
}

/// Pre-populate cache with API key -> AuthenticatedUser JSON
async fn seed_cache(cache: &Arc<dyn Cache>) {
    let entries = vec![
        ("pa_key", r#"{"user_id":"user-pa","tenant_id":null,"role":"PlatformAdmin"}"#),
        ("ta_key", r#"{"user_id":"user-ta1","tenant_id":"tenant-a","role":"TenantAdmin"}"#),
        ("pi_key", r#"{"user_id":"user-pi1","tenant_id":"tenant-a","role":"Pilot"}"#),
    ];
    for (k, v) in entries {
        cache.set(k, v.as_bytes(), Some(3600)).await.expect("cache set");
    }
}

/// Build a test server with seeded state (Postgres + cache + in-memory adapters).
async fn build_server() -> (TestServer, Arc<dyn Cache>, PgPool) {
    let pool = setup_pg().await;
    seed_data(&pool).await;

    let user_repo: Arc<dyn Repository> = Arc::new(InMemoryEventRepository::default());
    let tenant_repo: Arc<dyn Repository> = Arc::new(InMemoryEventRepository::default());
    let event_bus: Arc<dyn EventPublisher> = Arc::new(InMemoryEventBus::default());
    let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::default());

    let state = AppState {
        user_repo,
        tenant_repo,
        event_bus,
        cache: cache.clone(),
        pg_pool: Some(pool.clone()),
    };

    let app = create_app(state);
    let server = TestServer::new(app).expect("start test server");

    seed_cache(&cache).await;

    (server, cache, pool)
}


#[tokio::test]
async fn platform_admin_lists_all_tenants_and_users() {
    let (server, _cache, _pool) = build_server().await;

    let res_tenants = server
        .get("/api/tenants/list")
        .add_header(axum::http::header::AUTHORIZATION, HeaderValue::from_static("Bearer pa_key"))
        .await;
    assert_eq!(res_tenants.status_code(), StatusCode::OK);
    let body: Value = res_tenants.json();
    assert_eq!(body["data"].as_array().unwrap().len(), 2);

    let res_users = server
        .get("/api/users/list")
        .add_header(axum::http::header::AUTHORIZATION, HeaderValue::from_static("Bearer pa_key"))
        .await;
    assert_eq!(res_users.status_code(), StatusCode::OK);
    let body_users: Value = res_users.json();
    assert_eq!(body_users["data"].as_array().unwrap().len(), 4);
}

#[tokio::test]
async fn tenant_admin_lists_scoped_tenant_and_users() {
    let (server, _cache, _pool) = build_server().await;

    let res_tenant = server
        .get("/api/tenants/list")
        .add_header(axum::http::header::AUTHORIZATION, HeaderValue::from_static("Bearer ta_key"))
        .await;
    assert_eq!(res_tenant.status_code(), StatusCode::OK);
    let body_tenant: Value = res_tenant.json();
    assert_eq!(body_tenant["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        body_tenant["data"][0]["tenant_id"].as_str().unwrap(),
        "tenant-a"
    );

    let res_users = server
        .get("/api/users/list")
        .add_header(axum::http::header::AUTHORIZATION, HeaderValue::from_static("Bearer ta_key"))
        .await;
    assert_eq!(res_users.status_code(), StatusCode::OK);
    let body_users: Value = res_users.json();
    // TA sees own tenant's users (ta1 + pilot1) = 2
    assert_eq!(body_users["data"].as_array().unwrap().len(), 2);
    for u in body_users["data"].as_array().unwrap() {
        assert_eq!(u["tenant_id"].as_str().unwrap(), "tenant-a");
    }
}

#[tokio::test]
async fn pilot_lists_only_self_user_and_single_tenant() {
    let (server, _cache, _pool) = build_server().await;

    let res_tenant = server
        .get("/api/tenants/list")
        .add_header(axum::http::header::AUTHORIZATION, HeaderValue::from_static("Bearer pi_key"))
        .await;
    assert_eq!(res_tenant.status_code(), StatusCode::OK);
    let body_tenant: Value = res_tenant.json();
    assert_eq!(body_tenant["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        body_tenant["data"][0]["tenant_id"].as_str().unwrap(),
        "tenant-a"
    );

    let res_users = server
        .get("/api/users/list")
        .add_header(axum::http::header::AUTHORIZATION, HeaderValue::from_static("Bearer pi_key"))
        .await;
    assert_eq!(res_users.status_code(), StatusCode::OK);
    let body_users: Value = res_users.json();
    assert_eq!(body_users["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        body_users["data"][0]["user_id"].as_str().unwrap(),
        "user-pi1"
    );
}

#[tokio::test]
async fn unauthenticated_lists_are_rejected() {
    let (server, _cache, _pool) = build_server().await;

    let res = server.get("/api/users/list").await;
    assert_eq!(res.status_code(), StatusCode::UNAUTHORIZED);

    let res2 = server.get("/api/tenants/list").await;
    assert_eq!(res2.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn caching_returns_same_response_on_second_call() {
    let (server, _cache, _pool) = build_server().await;

    let first = server
        .get("/api/users/list")
        .add_header(axum::http::header::AUTHORIZATION, HeaderValue::from_static("Bearer pa_key"))
        .await;
    assert_eq!(first.status_code(), StatusCode::OK);
    let first_body: Value = first.json();
    let second = server
        .get("/api/users/list")
        .add_header(axum::http::header::AUTHORIZATION, HeaderValue::from_static("Bearer pa_key"))
        .await;
    assert_eq!(second.status_code(), StatusCode::OK);
    let second_body: Value = second.json();
    assert_eq!(first_body, second_body);
    // (Qualitative cache verification; deeper metric-based check could be added later)
}

#[tokio::test]
async fn tenant_admin_cannot_create_user_in_other_tenant() {
    let (server, _cache, _pool) = build_server().await;

    // Pre-create API key entry for tenant admin already done.
    // Attempt to register user with different tenant (tenant-b)
    let payload = serde_json::json!({
        "username":"intruder",
        "email":"intruder@example.com",
        "password_plaintext":"pw",
        "initial_role": 2,  // Pilot
        "tenant_id":"tenant-b"
    });

    let res = server
        .post("/api/users")
        .add_header(axum::http::header::AUTHORIZATION, HeaderValue::from_static("Bearer ta_key"))
        .json(&payload)
        .await;

    assert_eq!(res.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn pilot_cannot_register_user() {
    let (server, _cache, _pool) = build_server().await;

    let payload = serde_json::json!({
        "username":"newuser",
        "email":"newuser@example.com",
        "password_plaintext":"pw",
        "initial_role": 2,
        "tenant_id":"tenant-a"
    });

    let res = server
        .post("/api/users")
        .add_header(axum::http::header::AUTHORIZATION, HeaderValue::from_static("Bearer pi_key"))
        .json(&payload)
        .await;

    assert_eq!(res.status_code(), StatusCode::FORBIDDEN);
}
