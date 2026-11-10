use axum::extract::rejection::FormRejection;

use crate::error::{InternalError, into_response_via, oauth_code::OAuthErrorCode, page::PageError};

#[derive(Debug)]
pub enum LoginError {
    MalformedRequest,
    InvalidReturnTo,
    ServerError(InternalError),
}

impl From<LoginError> for PageError {
    fn from(error: LoginError) -> Self {
        match error {
            LoginError::MalformedRequest => {
                PageError::new(OAuthErrorCode::InvalidRequest, "The request is malformed.")
            }
            LoginError::InvalidReturnTo => PageError::new(
                OAuthErrorCode::InvalidRequest,
                "The return_to parameter is missing or is not a path on this server.",
            ),
            LoginError::ServerError(source) => PageError::Server(source),
        }
    }
}

into_response_via!(LoginError => PageError);

impl From<InternalError> for LoginError {
    fn from(error: InternalError) -> Self {
        LoginError::ServerError(error)
    }
}

impl From<FormRejection> for LoginError {
    fn from(_: FormRejection) -> Self {
        LoginError::MalformedRequest
    }
}
