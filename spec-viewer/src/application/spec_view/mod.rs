use std::sync::Arc;

use crate::application::{
    contract::{
        content_gateway::{ContentGateway, GetContentError},
        spec_gateway::{SpecGateway, SpecGatewayError},
    },
    diff::{Diff, diff},
    markdown::render_markdown,
};

mod detail;
mod selection;

pub use detail::{PatchInfo, ProposalInfo, SpecDetail};
pub use selection::{PatchScope, Selection, ViewMode};

#[derive(Debug, thiserror::Error)]
pub enum ViewSpecError {
    #[error("no such spec, version or proposal")]
    NotFound,
    #[error(transparent)]
    Provider(SpecGatewayError),
}

/// A spec as the detail page shows it.
#[derive(Debug)]
pub struct SpecView {
    pub spec: SpecDetail,
    /// `None` when the spec has no text yet.
    pub selection: Option<Selection>,
    pub mode: ViewMode,
    pub body: Body,
}

#[derive(Debug)]
pub enum Body {
    /// The spec has no text yet.
    Empty,
    /// Sanitized HTML.
    Rendered(String),
    Diff {
        /// What the selection is compared with, `None` for an empty text.
        base: Option<Selection>,
        diff: Diff,
    },
    /// The text could not be loaded from the content storage.
    Unavailable,
}

/// The use case of showing a spec with the text of one of its versions or proposals.
pub struct SpecViewService {
    spec_gateway: Arc<dyn SpecGateway>,
    content_gateway: Arc<dyn ContentGateway>,
}

impl SpecViewService {
    pub fn new(
        spec_gateway: Arc<dyn SpecGateway>,
        content_gateway: Arc<dyn ContentGateway>,
    ) -> Self {
        SpecViewService {
            spec_gateway,
            content_gateway,
        }
    }

    /// Loads the spec with the given id and the text of the selection.
    ///
    /// Without a selection, the latest version is shown. When the text can not be loaded the
    /// view is still returned, with [`Body::Unavailable`].
    pub async fn view_spec(
        &self,
        id: &str,
        selection: Option<Selection>,
        mode: ViewMode,
    ) -> Result<SpecView, ViewSpecError> {
        let spec = match self.spec_gateway.get_spec(id).await {
            Ok(kind) => SpecDetail::from(kind),
            // An id that is not even well formed is as unknown as one that doesn't exist.
            Err(SpecGatewayError::Status(status))
                if matches!(
                    status.code(),
                    tonic::Code::NotFound | tonic::Code::InvalidArgument
                ) =>
            {
                return Err(ViewSpecError::NotFound);
            }
            Err(error) => return Err(ViewSpecError::Provider(error)),
        };

        let selection = match selection {
            Some(selection) if spec.contains(&selection) => Some(selection),
            Some(_) => return Err(ViewSpecError::NotFound),
            None => spec.default_selection(),
        };

        let body = match &selection {
            Some(selection) => self.load_body(&spec, selection, mode).await,
            None => Body::Empty,
        };

        Ok(SpecView {
            spec,
            selection,
            mode,
            body,
        })
    }

    async fn load_body(&self, spec: &SpecDetail, selection: &Selection, mode: ViewMode) -> Body {
        let body = match mode {
            ViewMode::Rendered => self
                .text_at(spec.content_uri(selection))
                .await
                .map(|text| Body::Rendered(render_markdown(&text))),
            ViewMode::Diff => {
                let base = spec.base_of(selection);
                let (old, new) = tokio::join!(
                    self.text_at(base.as_ref().and_then(|base| spec.content_uri(base))),
                    self.text_at(spec.content_uri(selection)),
                );
                old.and_then(|old| {
                    Ok(Body::Diff {
                        base,
                        diff: diff(&old, &new?),
                    })
                })
            }
        };

        body.unwrap_or_else(|error| {
            tracing::error!(spec_id = %spec.id, %error, "failed to load the text of the spec");
            Body::Unavailable
        })
    }

    /// The text at `uri`, or an empty text without any.
    async fn text_at(&self, uri: Option<&str>) -> Result<String, GetContentError> {
        match uri {
            Some(uri) => self.content_gateway.get(uri).await,
            None => Ok(String::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::contract::{
        content_gateway::MockContentGateway, spec_gateway::MockSpecGateway,
    };
    use spec_api::{Patch, Spec, Version, get_spec_response::Kind};

    fn version(patches: &[&str]) -> Version {
        Version {
            id: String::new(),
            patches: patches
                .iter()
                .map(|uri| Patch {
                    id: uri.to_string(),
                    content_uri: uri.to_string(),
                    ..Default::default()
                })
                .collect(),
        }
    }

    /// Knows a published spec `spec` with version 1 with the patches `p1` and `p2`, a provider
    /// that is `down`, and no other spec.
    fn spec_gateway() -> MockSpecGateway {
        let mut gateway = MockSpecGateway::new();
        gateway.expect_get_spec().returning(|id| match id {
            "spec" => Ok(Kind::Published(Spec {
                id: id.into(),
                versions: vec![version(&["p1", "p2"])],
                ..Default::default()
            })),
            "down" => Err(SpecGatewayError::Status(tonic::Status::unavailable("down"))),
            _ => Err(SpecGatewayError::Status(tonic::Status::not_found(
                "no such spec",
            ))),
        });
        gateway
    }

    /// Has the texts `p1` and `p2`, and answers `404` for any other.
    fn content_gateway() -> MockContentGateway {
        let mut gateway = MockContentGateway::new();
        gateway.expect_get().returning(|uri| match uri {
            "p1" => Ok("# One\n".to_string()),
            "p2" => Ok("# Two\n".to_string()),
            _ => Err(GetContentError::Status(404, uri.to_string())),
        });
        gateway
    }

    fn service(
        spec_gateway: MockSpecGateway,
        content_gateway: MockContentGateway,
    ) -> SpecViewService {
        SpecViewService::new(Arc::new(spec_gateway), Arc::new(content_gateway))
    }

    async fn view_of(
        id: &str,
        selection: Option<Selection>,
        mode: ViewMode,
    ) -> Result<SpecView, ViewSpecError> {
        service(spec_gateway(), content_gateway())
            .view_spec(id, selection, mode)
            .await
    }

    #[tokio::test]
    async fn shows_the_latest_version_rendered_by_default() {
        let view = view_of("spec", None, ViewMode::Rendered).await.unwrap();

        assert_eq!(view.selection, Some(Selection::Version(1)));
        assert!(matches!(view.body, Body::Rendered(html) if html.contains("<h1>Two</h1>")));
    }

    #[tokio::test]
    async fn loads_the_diff_and_the_text_of_a_patch() {
        let view = view_of("spec", Some(Selection::Version(1)), ViewMode::Diff)
            .await
            .unwrap();
        assert!(matches!(&view.body, Body::Diff { base: None, diff } if diff.added == 1));

        let view = view_of(
            "spec",
            Some(Selection::Patch {
                scope: PatchScope::Version(1),
                number: 2,
            }),
            ViewMode::Rendered,
        )
        .await
        .unwrap();
        assert!(matches!(view.body, Body::Rendered(html) if html.contains("<h1>Two</h1>")));
    }

    #[tokio::test]
    async fn maps_unknown_specs_and_selections_to_not_found() {
        assert!(matches!(
            view_of("missing", None, ViewMode::Rendered).await,
            Err(ViewSpecError::NotFound)
        ));
        assert!(matches!(
            view_of("spec", Some(Selection::Version(2)), ViewMode::Rendered).await,
            Err(ViewSpecError::NotFound)
        ));
    }

    #[tokio::test]
    async fn does_not_load_any_text_for_an_unknown_selection() {
        let mut content_gateway = MockContentGateway::new();
        content_gateway.expect_get().never();

        let result = service(spec_gateway(), content_gateway)
            .view_spec("spec", Some(Selection::Version(2)), ViewMode::Rendered)
            .await;

        assert!(matches!(result, Err(ViewSpecError::NotFound)));
    }

    #[tokio::test]
    async fn maps_a_malformed_id_to_not_found() {
        let mut spec_gateway = MockSpecGateway::new();
        spec_gateway.expect_get_spec().returning(|_| {
            Err(SpecGatewayError::Status(tonic::Status::invalid_argument(
                "The provided spec ID is invalid.",
            )))
        });

        let result = service(spec_gateway, MockContentGateway::new())
            .view_spec("not-a-uuid", None, ViewMode::Rendered)
            .await;

        assert!(matches!(result, Err(ViewSpecError::NotFound)));
    }

    #[tokio::test]
    async fn maps_an_empty_provider_response_to_a_provider_error() {
        let mut spec_gateway = MockSpecGateway::new();
        spec_gateway
            .expect_get_spec()
            .returning(|_| Err(SpecGatewayError::MissingKind));

        let result = service(spec_gateway, MockContentGateway::new())
            .view_spec("spec", None, ViewMode::Rendered)
            .await;

        assert!(matches!(result, Err(ViewSpecError::Provider(_))));
    }

    #[tokio::test]
    async fn maps_an_unreachable_provider_to_a_provider_error() {
        assert!(matches!(
            view_of("down", None, ViewMode::Rendered).await,
            Err(ViewSpecError::Provider(_))
        ));
    }

    #[tokio::test]
    async fn still_shows_the_spec_when_its_text_is_unavailable() {
        let mut content_gateway = MockContentGateway::new();
        content_gateway
            .expect_get()
            .returning(|uri| Err(GetContentError::Request(format!("{uri} is unreachable"))));

        let view = service(spec_gateway(), content_gateway)
            .view_spec("spec", None, ViewMode::Rendered)
            .await
            .unwrap();

        assert!(matches!(view.body, Body::Unavailable));
    }
}
