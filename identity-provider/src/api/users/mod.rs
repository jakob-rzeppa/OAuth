use axum::{middleware, routing};

use crate::security::middleware::auth_middleware;

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
        .route(
            "/",
            routing::post(create::create_user_endpoint).layer(middleware::from_fn(
                auth_middleware("identity:users:create", false),
            )),
        )
        // This route is defined as a GET. With the next axum version, we use the QUERY method instead of GET for this endpoint.
        .route(
            "/",
            routing::get(query::query_users_endpoint).layer(middleware::from_fn(auth_middleware(
                "identity:users:query",
                false,
            ))),
        )
        .route(
            "/{user_id}",
            routing::delete(delete::delete_user_endpoint).layer(middleware::from_fn(
                auth_middleware("identity:users:delete", false),
            )),
        )
        .route(
            "/{user_id}",
            routing::get(get::get_user_endpoint).layer(middleware::from_fn(auth_middleware(
                "identity:users:get",
                false,
            ))),
        )
        .route(
            "/{user_id}",
            routing::patch(update::update_user_endpoint).layer(middleware::from_fn(
                auth_middleware("identity:users:update", false),
            )),
        )
        .route(
            "/{user_id}/password",
            routing::put(password::set::set_password_endpoint).layer(middleware::from_fn(
                auth_middleware("identity:users:password:set", false),
            )),
        )
        .route(
            "/{user_id}/password",
            routing::delete(password::reset::reset_password_endpoint).layer(middleware::from_fn(
                auth_middleware("identity:users:password:reset", false),
            )),
        )
        .route(
            "/authenticate",
            routing::post(authenticate::authenticate_user_endpoint).layer(middleware::from_fn(
                auth_middleware("identity:users:authenticate", false),
            )),
        )
}
