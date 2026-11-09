use crate::security::auth::error::AuthError;

mod auth;

struct AuthProgress {
    state: Option<auth::state::State>,
    code_verifier: Option<auth::code_verifier::CodeVerifier>,

    par_request: Option<auth::par::ParRequest>,
    par_response: Option<Result<auth::par::ParResponse, AuthError>>,

    auth_redirect_uri: Option<String>,
    auth_response: Option<Result<(), AuthError>>,

    token_request: Option<()>,
    token_response: Option<Result<(), AuthError>>,

    token: Option<String>,
}
