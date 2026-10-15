use axum::{Router, routing::get};

use crate::web::hello::hello_endpoint;

mod hello;

pub fn router() -> Router {
    Router::new().route("/", get(hello_endpoint))
}
