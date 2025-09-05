use argon2::{Argon2, PasswordHasher, password_hash::SaltString, password_hash::rand_core::OsRng};
use axum::Router;
use axum_test::TestServer;
use core_lib::{
    Cache, EventPublisher, Repository,
    adapters::{
        postgres_repository::PostgresEventRepository,
        in_memory_cache::InMemoryCache, in_memory_event_bus::InMemoryEventBus,
    },
};
use http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

// Import necessary items from the api_gateway crate
use api_gateway::{AppState, create_app};

#[derive(serde::Deserialize)]
struct LoginResponse {
    api_key: String,
    user_id: String,
}

// Helper function to set up the test application with in-memory dependencies
fn setup_test_app() -> TestServer {
    let user_repo: Arc<dyn Repository> = Arc::new(core_lib::adapters::in_memory_repository::InMemoryEventRepository::default());
    let tenant_repo: Arc<dyn Repository> = Arc::new(core_lib::adapters::in_memory_repository::InMemoryEventRepository::default());
    let event_bus: Arc<dyn EventPublisher> = Arc::new(InMemoryEventBus::default());
    let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::default());

    let app_state = AppState {
        user_repo: user_repo.clone(),
        tenant_repo: tenant_repo.clone(),
        event_bus: event_bus.clone(),
        cache: cache.clone(),
        pg_pool: None, // Tests don't use PostgreSQL, so this is None
        redis_client: None,
    };

    let app: Router = create_app(app_state);
    TestServer::new(app).expect("Failed to create TestServer")
}

// Helper function to set up the test application with PostgreSQL
async fn setup_test_app_with_postgres() -> (TestServer, testcontainers::ContainerAsync<Postgres>, PgPool) {
    let postgres_container = Postgres::default().start().await.unwrap();
    let connection_string = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        postgres_container.get_host_port_ipv4(5432).await.unwrap()
    );

    // Create connection pool
    let pg_pool = PgPool::connect(&connection_string).await.unwrap();

    // Run migrations
    sqlx::migrate!("./migrations").run(&pg_pool).await.unwrap();

    let user_repo: Arc<dyn Repository> = Arc::new(PostgresEventRepository::new(pg_pool.clone()));
    let tenant_repo: Arc<dyn Repository> = Arc::new(PostgresEventRepository::new(pg_pool.clone()));
    let event_bus: Arc<dyn EventPublisher> = Arc::new(InMemoryEventBus::default());
    let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::default());

    let app_state = AppState {
        user_repo: user_repo.clone(),
        tenant_repo: tenant_repo.clone(),
        event_bus: event_bus.clone(),
        cache: cache.clone(),
        pg_pool: Some(pg_pool.clone()),
        redis_client: None,
    };

    let app: Router = create_app(app_state);
    let server = TestServer::new(app).expect("Failed to create TestServer");

    (server, postgres_container, pg_pool)
}

#[tokio::test]
async fn test_login_endpoint_exists() {
    let server = setup_test_app();

    // Test that the login endpoint exists and accepts POST
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "testuser",
            "password": "password123"
        }))
        .await;

    // Should get 503 because no PostgreSQL pool in test setup
    // But this confirms the endpoint exists and accepts POST
    assert_eq!(login_response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_login_requires_postgres() {
    let server = setup_test_app();

    // Test that login requires PostgreSQL (which isn't available in tests)
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "testuser",
            "password": "password123"
        }))
        .await;

    // Should return 503 because PostgreSQL pool is None in test setup
    assert_eq!(login_response.status_code(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_bootstrap_status_check_with_no_users() {
    let server = setup_test_app();

    // Check if we can determine bootstrap status by trying to list users
    let users_response = server
        .get("/api/users/list?limit=1")
        .await;

    // Should return 401 since no auth and no bootstrap context
    assert_eq!(users_response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_bootstrap_status_check_requires_auth() {
    let server = setup_test_app();

    // Try to check users list without auth (bootstrap scenario)
    let users_response = server
        .get("/api/users/list?limit=1")
        .await;

    // Should return 401 because no auth header provided
    assert_eq!(users_response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_successful_login_with_correct_password() {
    let (server, _container, pool) = setup_test_app_with_postgres().await;

    // First, register a user
    let username = format!("testuser_{}", Uuid::new_v4());
    let email = format!("{}@test.com", username);
    let password = "correct_password_123";

    let register_response = server
        .post("/api/users")
        .json(&json!({
            "username": username,
            "email": email,
            "password_plaintext": password,
            "initial_role": 1 // PlatformAdmin
        }))
        .await;

    assert_eq!(register_response.status_code(), StatusCode::CREATED);

    // For testing, manually insert the user into the read model since projection worker isn't running
    let user_id = Uuid::new_v4().to_string(); // Generate a proper UUID
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    sqlx::query!(
        "INSERT INTO users (user_id, username, email, role, password_hash, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, NOW(), NOW())",
        user_id,
        username,
        format!("{}@test.com", username),
        "PlatformAdmin",
        password_hash
    )
    .execute(&pool)
    .await
    .unwrap();

    // Now try to login with correct password
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": username,
            "password": password
        }))
        .await;

    // Should succeed and return API key
    assert_eq!(login_response.status_code(), StatusCode::OK);

    let login_data: LoginResponse = login_response.json();
    assert!(!login_data.api_key.is_empty());
    assert!(!login_data.user_id.is_empty());
}

#[tokio::test]
async fn test_login_fails_with_wrong_password() {
    let (server, _container, _pool) = setup_test_app_with_postgres().await;

    // First, register a user
    let username = format!("testuser_{}", Uuid::new_v4());
    let email = format!("{}@test.com", username);
    let password = "correct_password_123";

    let register_response = server
        .post("/api/users")
        .json(&json!({
            "username": username,
            "email": email,
            "password_plaintext": password,
            "initial_role": 1 // PlatformAdmin
        }))
        .await;

    assert_eq!(register_response.status_code(), StatusCode::CREATED);

    // Try to login with wrong password
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": username,
            "password": "wrong_password_456"
        }))
        .await;

    // Should fail with 401 Unauthorized
    assert_eq!(login_response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_fails_with_nonexistent_user() {
    let (server, _container, _pool) = setup_test_app_with_postgres().await;

    // Try to login with a user that doesn't exist
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "nonexistent_user",
            "password": "some_password"
        }))
        .await;

    // Should fail with 401 Unauthorized
    assert_eq!(login_response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_requires_both_username_and_password() {
    let (server, _container, _pool) = setup_test_app_with_postgres().await;

    // Test missing username - Axum returns 422 for JSON validation errors
    let login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "password": "some_password"
        }))
        .await;

    assert_eq!(login_response.status_code(), StatusCode::UNPROCESSABLE_ENTITY);

    // Test missing password - Axum returns 422 for JSON validation errors
    let login_response2 = server
        .post("/api/auth/login")
        .json(&json!({
            "username": "some_user"
        }))
        .await;

    assert_eq!(login_response2.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
}
