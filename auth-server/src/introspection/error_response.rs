use axum::extract::rejection::FormRejection;

use crate::{
    error::{InternalError, api::ApiError, into_response_via},
    util::oauth_error::OAuthErrorCode,
};

#[derive(Debug)]
pub enum IntrospectionErrorResponse {
    InvalidRequestBody,
    ServerError(InternalError),
}

impl From<IntrospectionErrorResponse> for ApiError {
    fn from(error: IntrospectionErrorResponse) -> Self {
        match error {
            IntrospectionErrorResponse::InvalidRequestBody => ApiError::new(
                OAuthErrorCode::InvalidRequest,
                "The request is missing a required parameter, includes an invalid parameter value, or is otherwise malformed.",
            ),
            IntrospectionErrorResponse::ServerError(source) => ApiError::Server(source),
        }
    }
}

into_response_via!(IntrospectionErrorResponse => ApiError);

impl From<InternalError> for IntrospectionErrorResponse {
    fn from(error: InternalError) -> Self {
        IntrospectionErrorResponse::ServerError(error)
    }
}

impl From<FormRejection> for IntrospectionErrorResponse {
    fn from(_: FormRejection) -> Self {
        IntrospectionErrorResponse::InvalidRequestBody
    }
}
