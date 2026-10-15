use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    application::spec_list::{SpecListEntry, SpecListPage},
    state::AppState,
    web::display::{CssClass, Label, TimeDisplay, VersionLabel},
    web::{error::fragment::spec_list_unavailable_fragment, overview::ListParams, render, spec},
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

impl SpecListPartial {
    /// The path of the page of the spec of the entry.
    fn entry_path(&self, entry: &SpecListEntry) -> String {
        spec::page_path(&entry.id)
    }
}

/// The url of the page of the list starting at `page_token`, or of the first page.
pub(super) fn url(query: &ListParams, page_token: Option<&str>) -> String {
    match query.query_string_with_page_token(page_token).as_str() {
        "" => PATH.to_string(),
        query => format!("{PATH}?{query}"),
    }
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
            tracing::error!(%error, "failed to list specs");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_url_of_a_page_without_a_dangling_question_mark() {
        let all = ListParams::default();
        let search = ListParams {
            search: "bank".into(),
            ..Default::default()
        };

        assert_eq!(url(&all, None), "/spec-list");
        assert_eq!(url(&all, Some("7")), "/spec-list?page_token=7");
        assert_eq!(url(&search, None), "/spec-list?q=bank");
    }
}
