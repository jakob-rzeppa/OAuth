use api_macros::ApiErrorResponse;
use axum::extract::rejection::FormRejection;

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

impl From<FormRejection> for IntrospectionErrorResponse {
    fn from(_: FormRejection) -> Self {
        IntrospectionErrorResponse::InvalidRequestBody
    }
}
