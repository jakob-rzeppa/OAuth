use crate::{
    error::{
        InternalError,
        api::{ApiError, Challenge, Scheme},
        into_response_via,
    },
    util::oauth_error::OAuthErrorCode,
};

/// The errors of a protected resource (RFC 6750, section 3). Unlike the OAuth endpoints, they are
/// reported in the `WWW-Authenticate` header. The body repeats `error` and `error_description` as
/// JSON for clients that don't parse the header.
#[derive(Debug)]
pub enum UserInfoErrorResponse {
    /// No bearer token was sent. Only the authentication scheme is named, without an error code
    /// and with an empty body (RFC 6750, section 3.1).
    MissingToken,
    /// The token is unknown or expired, or its user no longer exists.
    InvalidToken,
    /// The token was not issued for OpenID Connect (no `openid` scope).
    InsufficientScope,
    ServerError(InternalError),
}

impl From<UserInfoErrorResponse> for ApiError {
    fn from(error: UserInfoErrorResponse) -> Self {
        match error {
            UserInfoErrorResponse::MissingToken => ApiError::Unauthenticated(Scheme::Bearer),
            UserInfoErrorResponse::InvalidToken => ApiError::challenged(
                OAuthErrorCode::InvalidToken,
                "The access token is invalid or expired.",
                Challenge::bearer(),
            ),
            UserInfoErrorResponse::InsufficientScope => ApiError::challenged(
                OAuthErrorCode::InsufficientScope,
                "The access token was not granted the openid scope.",
                Challenge::bearer().with_scope("openid"),
            ),
            UserInfoErrorResponse::ServerError(source) => ApiError::Server(source),
        }
    }
}

into_response_via!(UserInfoErrorResponse => ApiError);

impl From<InternalError> for UserInfoErrorResponse {
    fn from(error: InternalError) -> Self {
        UserInfoErrorResponse::ServerError(error)
    }
}
