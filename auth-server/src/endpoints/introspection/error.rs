use axum::extract::rejection::FormRejection;

use crate::error::{InternalError, api::ApiError, into_response_via, oauth_code::OAuthErrorCode};

#[derive(Debug)]
pub enum IntrospectionError {
    InvalidRequestBody,
    ServerError(InternalError),
}

impl From<IntrospectionError> for ApiError {
    fn from(error: IntrospectionError) -> Self {
        match error {
            IntrospectionError::InvalidRequestBody => ApiError::new(
                OAuthErrorCode::InvalidRequest,
                "The request is missing a required parameter, includes an invalid parameter value, or is otherwise malformed.",
            ),
            IntrospectionError::ServerError(source) => ApiError::Server(source),
        }
    }
}

into_response_via!(IntrospectionError => ApiError);

impl From<InternalError> for IntrospectionError {
    fn from(error: InternalError) -> Self {
        IntrospectionError::ServerError(error)
    }
}

impl From<FormRejection> for IntrospectionError {
    fn from(_: FormRejection) -> Self {
        IntrospectionError::InvalidRequestBody
    }
}
