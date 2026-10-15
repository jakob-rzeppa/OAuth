use askama::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::web::render;

/// Swapped in by htmx where a partial failed to load.
#[derive(Template)]
#[template(path = "error/partial/fragment.html")]
struct ErrorFragment<'a> {
    message: &'a str,
    /// Whether it replaces an item of a list, instead of the content of an element.
    is_list_item: bool,
}

pub(in crate::web) fn content_not_found_fragment() -> Response {
    into_content(render(
        StatusCode::NOT_FOUND,
        &ErrorFragment {
            message: "There is no such version or proposal. Pick another one in the sidebar.",
            is_list_item: false,
        },
    ))
}

pub(in crate::web) fn content_unavailable_fragment() -> Response {
    into_content(render(
        StatusCode::BAD_GATEWAY,
        &ErrorFragment {
            message: "The spec provider is not answering. Try again in a moment.",
            is_list_item: false,
        },
    ))
}

pub(in crate::web) fn spec_list_unavailable_fragment() -> Response {
    render(
        StatusCode::BAD_GATEWAY,
        &ErrorFragment {
            message: "The spec provider is not answering. Try again in a moment.",
            is_list_item: true,
        },
    )
}

/// Swaps the fragment into the target instead of replacing it, so the target is still
/// there for the next request.
fn into_content(response: Response) -> Response {
    ([("HX-Reswap", "innerHTML")], response).into_response()
}
