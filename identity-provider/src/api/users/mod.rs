pub mod authenticate;
pub mod create;
pub mod delete;
pub mod get;
pub mod password {
    pub mod reset;
    pub mod set;
}
pub mod query;
pub mod update;

/// v1/users
pub fn router() -> axum::Router {
    axum::Router::new()
        .route("/", axum::routing::post(create::create_user_endpoint))
        // This route is defined as a GET. With the next axum version, we use the QUERY method instead of GET for this endpoint.
        .route("/", axum::routing::get(query::query_users_endpoint))
        .route(
            "/{user_id}",
            axum::routing::delete(delete::delete_user_endpoint),
        )
        .route("/{user_id}", axum::routing::get(get::get_user_endpoint))
        .route(
            "/{user_id}",
            axum::routing::patch(update::update_user_endpoint),
        )
        .route(
            "/{user_id}/password",
            axum::routing::put(password::set::set_password_endpoint),
        )
        .route(
            "/{user_id}/password",
            axum::routing::delete(password::reset::reset_password_endpoint),
        )
        .route(
            "/authenticate",
            axum::routing::post(authenticate::authenticate_user_endpoint),
        )
}
