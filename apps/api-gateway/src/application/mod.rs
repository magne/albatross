use core_lib::CoreError;

// Declare sub-modules within the application layer
pub mod commands;
pub mod middleware; // Added
// pub mod queries; // Add later

// Define a top-level error type for the application layer
#[derive(thiserror::Error, Debug)]
#[allow(dead_code)]
pub enum ApplicationError {
    #[error("Core Error: {0}")]
    Core(#[from] CoreError), // Can wrap core errors

    #[error("Configuration Error: {0}")]
    Configuration(String),

    #[error("Authorization Error: {0}")]
    Unauthorized(String),

    #[error("Validation Error: {0}")]
    Validation(String),

    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Internal Application Error: {0}")]
    Internal(String),
    // Add other application-specific error variants as needed
}

// Optional: Implement conversion from specific handler errors if needed later
