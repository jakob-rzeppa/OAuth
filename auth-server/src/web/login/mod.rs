use axum::routing::get;

use crate::web::login::{page::login_page_endpoint, submit::login_submit_endpoint};

mod cookie;
mod error_response;
mod page;
mod return_to;
mod submit;

pub fn router() -> axum::Router {
    axum::Router::new().route(
        "/login",
        get(login_page_endpoint).post(login_submit_endpoint),
    )
}
