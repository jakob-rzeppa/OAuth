use axum::extract::FromRequestParts;
use serde::Deserialize;

use crate::web::login::error_response::LoginErrorResponse;

#[derive(Deserialize)]
pub struct LoginPageQuery {
    pub return_to: String,
}

impl<S: Send + Sync> FromRequestParts<S> for LoginPageQuery {
    type Rejection = LoginErrorResponse;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let query = parts
            .uri
            .query()
            .ok_or(LoginErrorResponse::InvalidReturnTo)?;
        serde_urlencoded::from_str(query).map_err(|_| LoginErrorResponse::MalformedRequest)
    }
}
