mod config;
pub mod context;
mod error_response;
mod introspect;
pub mod middleware;

/// Initializes the auth library. This must be called before using any other functions in this library.
pub fn init() {
    config::init();
}
