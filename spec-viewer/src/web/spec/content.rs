use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    application::spec_view::{Body, ViewSpecError},
    state::AppState,
    web::{
        display::{CssClass, Label, Sign, TimeDisplay},
        error::fragment::{content_not_found_fragment, content_unavailable_fragment},
        render,
        spec::{SpecViewModel, ViewParams},
    },
};

/// The text, or the diff, of the selected version or proposal, with the selection panel
/// of the sidebar replaced out of band.
#[derive(Template)]
#[template(path = "spec/partial/content_with_selection.html")]
struct ContentPartial<'a> {
    view: SpecViewModel<'a>,
}

pub(super) async fn content_partial(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ViewParams>,
) -> Response {
    let Ok((selection, mode)) = params.parse() else {
        return content_not_found_fragment();
    };
    let view = match state
        .spec_view_service
        .view_spec(&id, selection, mode)
        .await
    {
        Ok(view) => view,
        Err(ViewSpecError::NotFound) => return content_not_found_fragment(),
        Err(ViewSpecError::Provider(error)) => {
            tracing::error!(spec_id = %id, %error, "failed to get spec");
            return content_unavailable_fragment();
        }
    };

    let view = SpecViewModel::new(&view);
    (
        [("HX-Push-Url", view.current_url())],
        render(StatusCode::OK, &ContentPartial { view }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::application::{
        contract::{content_gateway::MockContentGateway, spec_gateway::MockSpecGateway},
        spec_list::SpecListService,
        spec_view::SpecViewService,
    };
    use crate::web::spec::{CONTENT_ROUTE, proposal_diff_view};
    use axum::{Router, body::Body as HttpBody, http::Request, routing::get};
    use spec_api::{Patch, Spec, Version, get_spec_response::Kind};
    use tower::ServiceExt;

    /// A published spec `spec` with the version 1 with the patch `p1`, whose text is `# One`.
    fn router() -> Router {
        let mut spec_gateway = MockSpecGateway::new();
        spec_gateway.expect_get_spec().returning(|id| match id {
            "spec" => Ok(Kind::Published(Spec {
                id: id.into(),
                versions: vec![Version {
                    id: String::new(),
                    patches: vec![Patch {
                        content_uri: "p1".into(),
                        ..Default::default()
                    }],
                }],
                ..Default::default()
            })),
            _ => Err(
                crate::application::contract::spec_gateway::SpecGatewayError::Status(
                    tonic::Status::not_found("no such spec"),
                ),
            ),
        });
        let mut content_gateway = MockContentGateway::new();
        content_gateway
            .expect_get()
            .returning(|_| Ok("# One\n".to_string()));
        let spec_gateway = Arc::new(spec_gateway);
        let state = AppState::new(
            SpecViewService::new(spec_gateway.clone(), Arc::new(content_gateway)),
            SpecListService::new(spec_gateway),
        );

        Router::new()
            .route(CONTENT_ROUTE, get(content_partial))
            .with_state(state)
    }

    async fn get_content(uri: &str) -> (StatusCode, Option<String>, String) {
        let response = router()
            .oneshot(Request::get(uri).body(HttpBody::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let push_url = response
            .headers()
            .get("HX-Push-Url")
            .map(|url| url.to_str().unwrap().to_string());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, push_url, String::from_utf8(body.to_vec()).unwrap())
    }

    #[test]
    fn renders_the_diff_escaped() {
        let view = proposal_diff_view();

        let html = ContentPartial {
            view: SpecViewModel::new(&view),
        }
        .render()
        .unwrap();

        assert!(html.contains(r#"<article id="content""#), "{html}");
        assert!(html.contains("Proposal by alice"), "{html}");
        assert!(html.contains("Compared with an empty text"), "{html}");
        assert!(html.contains(r#"<tr class="removed">"#), "{html}");
        assert!(html.contains("old &#60;b&#62;"), "{html}");
        assert!(html.contains(r#"<tr class="added">"#), "{html}");
    }

    #[test]
    fn replaces_the_selection_panel_out_of_band() {
        let view = proposal_diff_view();

        let html = ContentPartial {
            view: SpecViewModel::new(&view),
        }
        .render()
        .unwrap();

        assert!(html.contains(r#"<div id="selection""#), "{html}");
        assert!(html.contains(r#"hx-swap-oob="true""#), "{html}");
        assert!(html.contains(r#"aria-current="page""#), "{html}");
    }

    #[tokio::test]
    async fn answers_with_the_content_and_the_url_to_push() {
        let (status, push_url, body) = get_content("/specs/spec/content?version=1").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(push_url.as_deref(), Some("/specs/spec?version=1"));
        assert!(body.contains("<h1>One</h1>"), "{body}");
        assert!(body.contains(r#"hx-swap-oob="true""#), "{body}");
    }

    #[tokio::test]
    async fn answers_with_an_error_fragment_for_an_unknown_spec_or_url() {
        for uri in ["/specs/missing/content", "/specs/spec/content?version=x"] {
            let (status, push_url, body) = get_content(uri).await;

            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
            assert_eq!(push_url, None, "{uri}");
            assert!(body.contains("status-message error"), "{body}");
        }
    }
}
