use api_macros::ApiErrorResponse;
use axum::extract::rejection::JsonRejection;

use crate::{error::InternalError, util::oauth_error::OAuthErrorCode};

#[ApiErrorResponse]
pub enum IntrospectionErrorResponse {
    #[code(OAuthErrorCode::InvalidRequest)]
    #[description(
        "The request is missing a required parameter, includes an invalid parameter value, or is otherwise malformed."
    )]
    InvalidRequestBody,

    #[server_error]
    ServerError(InternalError),
}

impl From<JsonRejection> for IntrospectionErrorResponse {
    fn from(_: JsonRejection) -> Self {
        IntrospectionErrorResponse::InvalidRequestBody
    }
}
