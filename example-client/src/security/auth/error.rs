/// The error body the auth-server sends with a rejected request.
#[derive(Debug, serde::Deserialize)]
struct ErrorBody {
    error: String,
    error_description: Option<String>,
}

#[derive(Debug)]
pub enum AuthError {
    /// The auth-server rejected the request with an OAuth error.
    Rejected {
        status_code: u16,
        /// The OAuth error code, e.g. `invalid_request`.
        error: String,
        error_description: Option<String>,
    },
    /// The auth-server answered with an error status but no OAuth error body, e.g. a 401 without
    /// credentials, or a response that didn't come from the auth-server (a proxy).
    UnexpectedResponse { status_code: u16 },
    /// The request couldn't be sent or its response couldn't be read.
    RequestError(reqwest::Error),
    /// The auth-server redirected back with an error instead of a code (RFC 6749 section 4.1.2.1),
    /// e.g. `access_denied` when the user declined.
    AuthorizationDenied {
        error: String,
        error_description: Option<String>,
    },
    /// The callback's `state` doesn't match the one sent, so it doesn't answer this request.
    StateMismatch,
    /// The callback's `iss` isn't the auth-server the request went to (RFC 9207).
    IssuerMismatch {
        expected: String,
        actual: Option<String>,
    },
    /// The callback carries neither a code nor an error.
    MissingCode,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Rejected {
                status_code,
                error,
                error_description,
            } => {
                write!(f, "HTTP {status_code}: {error}")?;
                if let Some(description) = error_description {
                    write!(f, " ({description})")?;
                }
                Ok(())
            }
            AuthError::UnexpectedResponse { status_code } => {
                write!(f, "HTTP {status_code} without an OAuth error body")
            }
            AuthError::RequestError(error) => write!(f, "request failed: {error}"),
            AuthError::AuthorizationDenied {
                error,
                error_description,
            } => {
                write!(f, "authorization failed: {error}")?;
                if let Some(description) = error_description {
                    write!(f, " ({description})")?;
                }
                Ok(())
            }
            AuthError::StateMismatch => {
                write!(f, "the returned state does not match the one that was sent")
            }
            AuthError::IssuerMismatch { expected, actual } => write!(
                f,
                "the returned iss {} is not the expected issuer {expected}",
                actual.as_deref().unwrap_or("(missing)")
            ),
            AuthError::MissingCode => write!(f, "the callback carries no authorization code"),
        }
    }
}

impl From<reqwest::Error> for AuthError {
    fn from(error: reqwest::Error) -> Self {
        AuthError::RequestError(error)
    }
}

impl AuthError {
    /// Passes a successful response through and turns an error status into an [`AuthError`].
    pub async fn check(response: reqwest::Response) -> Result<reqwest::Response, AuthError> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let status_code = status.as_u16();
        let body = response.bytes().await?;
        match serde_json::from_slice::<ErrorBody>(&body) {
            Ok(ErrorBody {
                error,
                error_description,
            }) => Err(AuthError::Rejected {
                status_code,
                error,
                error_description,
            }),
            Err(_) => Err(AuthError::UnexpectedResponse { status_code }),
        }
    }
}
