use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
};
use serde::de::DeserializeOwned;

use crate::admin::layout::notice::ErrorNotice;

/// A urlencoded form body. Unlike `axum::Form`, it collects repeated keys (checkboxes) into a `Vec`.
pub struct HtmlForm<T>(pub T);

impl<T, S> FromRequest<S> for HtmlForm<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ErrorNotice;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(request, state)
            .await
            .map_err(|_| ErrorNotice::invalid_form())?;

        serde_html_form::from_bytes(&bytes)
            .map(HtmlForm)
            .map_err(|_| ErrorNotice::invalid_form())
    }
}
