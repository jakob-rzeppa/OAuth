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
