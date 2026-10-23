use axum::middleware;

use auth_client::middleware::auth_middleware;

pub mod query;

// v1/roles
pub fn router() -> axum::Router {
    axum::Router::new().route(
        "/",
        axum::routing::get(query::query_roles_endpoint).layer(middleware::from_fn(
            auth_middleware("identity:roles:query", false),
        )),
    )
}
