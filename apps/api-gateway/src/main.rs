use axum::{
    Router,
    body::Body,
    http::{HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use rust_embed::RustEmbed;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// Define the struct to embed frontend assets
// Assumes the frontend build output is in `../web-ui/dist` relative to this crate's Cargo.toml
#[derive(RustEmbed)]
#[folder = "../web-ui/dist/"]
#[include = "*.html"]
#[include = "*.svg"]
#[include = "assets/*"]
struct FrontendAssets;

async fn static_asset(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    serve_embedded_file(path).await
}

async fn serve_index() -> impl IntoResponse {
    serve_embedded_file("index.html").await
}

async fn serve_embedded_file(path: &str) -> Response {
    match FrontendAssets::get(path) {
        Some(content) => {
            let mime_type = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(
                    header::CONTENT_TYPE,
                    HeaderValue::from_str(mime_type.as_ref()).unwrap(),
                )
                .body(Body::from(content.data))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("404 Not Found"))
            .unwrap(),
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing (logging)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Define the Axum router
    let app = Router::new()
        // Route for static assets (like CSS, JS, images)
        .route("/assets/{*path}", get(static_asset))
        // Route for the main index.html
        .route("/", get(serve_index))
        // Fallback route for SPA (Single Page Application) routing - serves index.html
        // This allows React Router to handle client-side routing
        .fallback(get(serve_index));

    // Define the address and port to run the server on
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("API Gateway listening on {}", addr);

    // Create a TCP listener
    let listener = TcpListener::bind(addr).await.unwrap();

    // Run the server
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
