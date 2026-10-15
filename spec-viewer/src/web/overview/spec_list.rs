use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    application::spec_list::SpecListPage,
    state::AppState,
    web::display::{CssClass, Label, TimeDisplay, VersionLabel},
    web::overview::ListParams,
    web::{error::fragment::spec_list_unavailable_fragment, render},
};

pub(super) const PATH: &str = "/spec-list";

#[derive(Deserialize)]
pub(super) struct Params {
    #[serde(flatten)]
    query: ListParams,
    page_token: Option<String>,
}

/// One page of specs. The first page replaces the list, later ones replace the
/// "Load more" button they were loaded by.
#[derive(Template)]
#[template(path = "overview/partial/spec_list.html")]
struct SpecListPartial {
    page: SpecListPage,
    is_first_page: bool,
    next_page_url: Option<String>,
}

pub(super) fn url(query: &ListParams, page_token: Option<&str>) -> String {
    format!("{PATH}?{}", query.query_string_with_page_token(page_token))
}

pub(super) async fn spec_list_partial(
    State(state): State<AppState>,
    Query(params): Query<Params>,
) -> Response {
    let is_first_page = params.page_token.is_none();
    let page = match state
        .spec_list_service
        .list_page(&params.query.query(), params.page_token)
        .await
    {
        Ok(page) => page,
        Err(error) => {
            eprintln!("failed to list specs: {error}");
            return spec_list_unavailable_fragment();
        }
    };

    let next_page_url = page
        .next_page_token
        .as_deref()
        .map(|token| url(&params.query, Some(token)));
    let response = render(
        StatusCode::OK,
        &SpecListPartial {
            page,
            is_first_page,
            next_page_url,
        },
    );

    if !is_first_page {
        return response;
    }
    // Keeps the search and filters in the url of the overview, without adding history
    // entries while typing.
    let query = params.query.query_string();
    let overview_url = if query.is_empty() {
        "/".to_string()
    } else {
        format!("/?{query}")
    };
    ([("HX-Replace-Url", overview_url)], response).into_response()
}
