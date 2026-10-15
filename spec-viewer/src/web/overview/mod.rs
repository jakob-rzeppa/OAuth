use askama::Template;
use axum::{Router, extract::Query, http::StatusCode, response::Response, routing::get};

use crate::{application::spec_list::ListQuery, state::AppState, web::render};

mod spec_list;

/// The search and filters, with the specs loaded by the [`spec_list`] partial.
#[derive(Template)]
#[template(path = "overview/page.html")]
struct OverviewPage {
    query: ListQuery,
    spec_list_url: String,
}

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(overview_page))
        .route(spec_list::PATH, get(spec_list::spec_list_partial))
}

async fn overview_page(Query(query): Query<ListQuery>) -> Response {
    let spec_list_url = spec_list::url(&query, None);
    render(
        StatusCode::OK,
        &OverviewPage {
            query,
            spec_list_url,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{
        contract::{content_gateway::MockContentGateway, spec_gateway::MockSpecGateway},
        spec_list::SpecListService,
        spec_view::SpecViewService,
    };
    use axum::{body::Body, http::Request};
    use std::sync::Arc;
    use tower::ServiceExt;

    /// The overview page itself calls no service, so the mocks expect no call.
    fn state() -> AppState {
        let spec_gateway = Arc::new(MockSpecGateway::new());
        AppState::new(
            SpecViewService::new(spec_gateway.clone(), Arc::new(MockContentGateway::new())),
            SpecListService::new(spec_gateway),
        )
    }

    #[tokio::test]
    async fn keeps_the_url_query_in_the_form_and_the_list_url() {
        let response = router()
            .with_state(state())
            .oneshot(
                Request::get("/?q=bank&kind=draft&order=oldest")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains(r#"value="bank""#), "{body}");
        assert!(
            body.contains(r#"<option value="draft" selected>"#),
            "{body}"
        );
        assert!(
            body.contains(r#"<option value="oldest" selected>"#),
            "{body}"
        );
        assert!(
            body.contains("/spec-list?q=bank&#38;kind=draft&#38;order=oldest"),
            "{body}"
        );
    }
}
