use askama::Template;
use axum::{http::StatusCode, response::Response};

use crate::web::render;

pub(super) mod fragment;

/// A full page explaining what went wrong.
#[derive(Template)]
#[template(path = "error/page.html")]
struct ErrorPage<'a> {
    title: &'a str,
    message: &'a str,
}

pub(super) fn not_found_page() -> Response {
    render(
        StatusCode::NOT_FOUND,
        &ErrorPage {
            title: "Spec not found",
            message: "There is no spec, version or proposal at this address. It may have been removed, or the link is incomplete.",
        },
    )
}

pub(super) fn provider_unavailable_page() -> Response {
    render(
        StatusCode::BAD_GATEWAY,
        &ErrorPage {
            title: "Specs can't be loaded right now",
            message: "The spec provider is not answering. Reload the page in a moment.",
        },
    )
}
