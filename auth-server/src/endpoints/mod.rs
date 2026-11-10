//! This module contains all the endpoints for the auth server.
//!
//! The code inside each endpoint module should be opaque to the rest of the server
//! and each other, and only expose a `router()` function that returns
//! an `axum::Router` with all the routes of the endpoint.
//! The router is then merged into the global router in this module.

use axum::Router;

mod authorize;
mod discovery;
mod introspection;
pub mod jwks;
mod login;
mod par;
mod token;
mod userinfo;

pub fn router() -> Router {
    Router::new()
        .merge(par::router())
        .merge(token::router())
        .merge(introspection::router())
        .merge(discovery::router())
        .merge(jwks::router())
        .merge(userinfo::router())
        .merge(authorize::router())
        .merge(login::router())
}
