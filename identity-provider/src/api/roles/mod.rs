pub mod query;

// v1/roles
pub fn router() -> axum::Router {
    axum::Router::new().route("/", axum::routing::get(query::query_roles_endpoint))
}
