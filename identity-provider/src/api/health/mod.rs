use crate::api::health::response::HealthResponse;

pub mod response;

/// Liveness probe for the container healthcheck.
///
/// Deliberately does not touch the database - it answers "is the HTTP server
/// accepting requests", which is what dependent services gate their startup on.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses((status = OK, description = "The server is accepting requests.", body = HealthResponse)),
)]
#[axum::debug_handler]
pub async fn health_endpoint() -> HealthResponse {
    HealthResponse {
        status: "ok".to_string(),
    }
}
