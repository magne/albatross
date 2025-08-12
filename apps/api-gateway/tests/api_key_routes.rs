use axum::Router;
use axum_test::TestServer;
use core_lib::{
    Cache, EventPublisher, Repository,
    adapters::{
        in_memory_cache::InMemoryCache, in_memory_event_bus::InMemoryEventBus,
        in_memory_repository::InMemoryEventRepository,
    },
};
use http::{HeaderName, HeaderValue, StatusCode}; // Added HeaderName, HeaderValue
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

// Import necessary items from the api_gateway crate
// Note: We might need to adjust visibility (pub) in api_gateway/src/main.rs or lib.rs if needed
use api_gateway::{AppState, GenerateApiKeyResponse, create_app}; // Assuming these are made public or accessible

// Helper function to set up the test application with in-memory dependencies
fn setup_test_app() -> TestServer {
    let user_repo: Arc<dyn Repository> = Arc::new(InMemoryEventRepository::default());
    let tenant_repo: Arc<dyn Repository> = Arc::new(InMemoryEventRepository::default());
    let event_bus: Arc<dyn EventPublisher> = Arc::new(InMemoryEventBus::default());
    let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::default());

    let app_state = AppState {
        user_repo: user_repo.clone(),
        tenant_repo: tenant_repo.clone(),
        event_bus: event_bus.clone(),
        cache: cache.clone(),
        pg_pool: None,
        redis_client: None,
    };

    let app: Router = create_app(app_state);
    TestServer::new(app).expect("Failed to create TestServer")
}

#[tokio::test]
async fn test_generate_and_revoke_api_key() {
    let server = setup_test_app();

    // 1. Register a user first to get a user_id
    let username = format!("testuser_{}", Uuid::new_v4());
    let email = format!("{}@test.com", username);
    let password = "password123";

    let register_response = server
        .post("/api/users")
        .json(&json!({
            "username": username,
            "email": email,
            "password_plaintext": password, // Correct field name
            "initial_role": 1 // Use PlatformAdmin (1) which doesn't require tenant_id
            // tenant_id is omitted, which is correct for PlatformAdmin
        }))
        .await;

    assert_eq!(register_response.status_code(), StatusCode::CREATED);
    let user_id = register_response
        .json::<serde_json::Value>()
        .get("user_id")
        .expect("Response should contain user_id")
        .as_str()
        .expect("user_id should be a string")
        .to_string();

    // 2. Generate an API key for the user
    let key_name = "My Test Key";
    let generate_response = server
        .post(&format!("/api/users/{}/apikeys", user_id))
        .json(&json!({ "key_name": key_name }))
        .await;

    assert_eq!(generate_response.status_code(), StatusCode::OK);
    let generated_key_data = generate_response.json::<GenerateApiKeyResponse>();

    assert!(!generated_key_data.key_id.is_empty());
    assert!(generated_key_data.key_id.starts_with("key_"));
    assert!(!generated_key_data.api_key.is_empty());
    assert_eq!(generated_key_data.api_key.len(), 32); // Assuming 32 char length

    let key_id = generated_key_data.key_id;
    let _api_key = generated_key_data.api_key; // Store if needed for auth test later

    // 3. Revoke the generated API key (authorized)
    let revoke_response = server
        .delete(&format!("/api/users/{}/apikeys/{}", user_id, key_id))
        .add_header(
            http::HeaderName::from_static("authorization"),
            http::HeaderValue::from_str(&format!("Bearer {}", _api_key)).unwrap(),
        )
        .await;

    assert_eq!(revoke_response.status_code(), StatusCode::NO_CONTENT);

    // 4. (Optional) Try to revoke again - should likely return 404 or maybe still 204/error depending on impl
    let revoke_again_response = server
        .delete(&format!("/api/users/{}/apikeys/{}", user_id, key_id))
        .add_header(
            http::HeaderName::from_static("authorization"),
            http::HeaderValue::from_str(&format!("Bearer {}", _api_key)).unwrap(),
        )
        .await;
    // Asserting 404 as the key is likely gone from the aggregate state after revoke event
    assert_eq!(revoke_again_response.status_code(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_key_authentication() {
    let server = setup_test_app();

    // 1. Register a user
    let username = format!("authuser_{}", Uuid::new_v4());
    let email = format!("{}@test.com", username);
    let password = "password123";
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
    let user_id = register_response.json::<serde_json::Value>()["user_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. Generate an API key
    let key_name = "Auth Test Key";
    let generate_response = server
        .post(&format!("/api/users/{}/apikeys", user_id))
        .json(&json!({ "key_name": key_name }))
        .await;
    assert_eq!(generate_response.status_code(), StatusCode::OK);
    let generated_key_data = generate_response.json::<GenerateApiKeyResponse>();
    let key_id = generated_key_data.key_id;
    let api_key = generated_key_data.api_key;

    // 3. Try accessing protected route WITHOUT auth
    let protected_no_auth = server.get("/api/protected").await;
    assert_eq!(protected_no_auth.status_code(), StatusCode::UNAUTHORIZED);

    // 4. Try accessing protected route WITH VALID auth
    let auth_header_value: HeaderValue = HeaderValue::from_str(&format!("Bearer {}", api_key))
        .expect("Failed to create auth header value"); // Explicit type annotation
    let protected_with_auth = server
        .get("/api/protected")
        .add_header(
            HeaderName::from_static("authorization"),
            auth_header_value.clone(),
        ) // Use explicit types
        .await;
    assert_eq!(protected_with_auth.status_code(), StatusCode::OK);
    let response_text = protected_with_auth.text(); // Get text before asserting
    println!("Protected route response: '{}'", response_text); // Print the response
    assert!(
        response_text.contains(&user_id),
        "Response text should contain user_id"
    ); // Check if response contains user_id

    // 5. Revoke the key
    let revoke_response = server
        .delete(&format!("/api/users/{}/apikeys/{}", user_id, key_id))
        .add_header(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
        )
        .await;
    assert_eq!(revoke_response.status_code(), StatusCode::NO_CONTENT);

    // 6. Try accessing protected route WITH REVOKED auth
    // Re-use the header value created earlier
    let protected_revoked_auth = server
        .get("/api/protected")
        .add_header(HeaderName::from_static("authorization"), auth_header_value) // Use explicit types
        .await;
    assert_eq!(
        protected_revoked_auth.status_code(),
        StatusCode::UNAUTHORIZED
    );
}

// TODO: Add test for generating key for non-existent user (expect 404)
// TODO: Add test for revoking key for non-existent user (expect 404)
// TODO: Add test for revoking non-existent key_id (expect 404)
// TODO: Add test for API key authentication middleware (/protected route)
