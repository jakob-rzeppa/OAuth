use axum::{
    Form,
    extract::{FromRequest, Request},
    http::request::Parts,
};
use serde::de::DeserializeOwned;

/// Deserializes the query string of the request. A missing query string counts as empty.
pub fn parse_query<T: DeserializeOwned>(parts: &Parts) -> Option<T> {
    serde_urlencoded::from_str(parts.uri.query().unwrap_or_default()).ok()
}

/// Deserializes an `application/x-www-form-urlencoded` request body.
pub async fn parse_form<T: DeserializeOwned, S: Send + Sync>(
    request: Request,
    state: &S,
) -> Option<T> {
    Form::<T>::from_request(request, state)
        .await
        .map(|Form(form)| form)
        .ok()
}
