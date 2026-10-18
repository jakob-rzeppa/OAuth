use axum::extract::rejection::FormRejection;

use crate::{
    error::{InternalError, into_response_via, page::PageError},
    util::oauth_error::OAuthErrorCode,
};

#[derive(Debug)]
pub enum LoginErrorResponse {
    MalformedRequest,
    InvalidReturnTo,
    ServerError(InternalError),
}

impl From<LoginErrorResponse> for PageError {
    fn from(error: LoginErrorResponse) -> Self {
        match error {
            LoginErrorResponse::MalformedRequest => PageError::new(
                OAuthErrorCode::InvalidRequest,
                "The request is malformed.",
            ),
            LoginErrorResponse::InvalidReturnTo => PageError::new(
                OAuthErrorCode::InvalidRequest,
                "The return_to parameter is missing or is not a path on this server.",
            ),
            LoginErrorResponse::ServerError(source) => PageError::Server(source),
        }
    }
}

into_response_via!(LoginErrorResponse => PageError);

impl From<InternalError> for LoginErrorResponse {
    fn from(error: InternalError) -> Self {
        LoginErrorResponse::ServerError(error)
    }
}

impl From<FormRejection> for LoginErrorResponse {
    fn from(_: FormRejection) -> Self {
        LoginErrorResponse::MalformedRequest
    }
}
