use crate::{
    config,
    security::auth::{
        authorization::{AuthorizationResponse, CallbackQuery, authorization_url},
        code_verifier::CodeVerifier,
        error::AuthError,
        par::{ParRequest, ParResponse, register_par},
        state::State,
        token::{TokenRequest, TokenResponse, request_token},
    },
};

pub mod auth;
pub mod session;

/// How far a session got through the authorization code flow (PAR + PKCE).
/// Each step fills its slots; a step only runs once the previous one succeeded.
#[derive(Default)]
pub struct AuthProgress {
    pub state: Option<State>,
    pub code_verifier: Option<CodeVerifier>,

    pub par_request: Option<ParRequest>,
    pub par_response: Option<Result<ParResponse, AuthError>>,

    pub auth_redirect_uri: Option<String>,
    pub callback_query: Option<CallbackQuery>,
    pub auth_response: Option<Result<AuthorizationResponse, AuthError>>,

    pub token_request: Option<TokenRequest>,
    pub token_response: Option<Result<TokenResponse, AuthError>>,

    pub token: Option<String>,
}

impl AuthProgress {
    /// Step 1: generates the state and the PKCE verifier, locally.
    pub fn prepare(&mut self) {
        if self.state.is_some() {
            return;
        }
        self.state = Some(State::generate());
        self.code_verifier = Some(CodeVerifier::generate());
    }

    /// Step 2: pushes the authorization request to the auth-server, and with its `request_uri`
    /// builds the URL the browser is sent to.
    pub async fn push_par(&mut self) {
        let (Some(state), Some(code_verifier)) = (&self.state, &self.code_verifier) else {
            return;
        };
        if matches!(self.par_response, Some(Ok(_))) {
            return;
        }

        let request = ParRequest::new(state, code_verifier);
        let response = register_par(&request).await;
        self.auth_redirect_uri = response
            .as_ref()
            .ok()
            .map(|response| authorization_url(response.request_uri()));
        self.par_request = Some(request);
        self.par_response = Some(response);
    }

    /// Step 3: the browser comes back from the auth-server. Checks the state and the issuer.
    pub fn handle_callback(&mut self, query: CallbackQuery) {
        let Some(expected_state) = &self.state else {
            return;
        };
        if self.auth_redirect_uri.is_none() || matches!(self.auth_response, Some(Ok(_))) {
            return;
        }

        self.auth_response = Some(check_callback(&query, expected_state));
        self.callback_query = Some(query);
    }

    /// Step 4: exchanges the code for tokens.
    pub async fn exchange_code(&mut self) {
        let (Some(Ok(authorization)), Some(code_verifier)) =
            (&self.auth_response, &self.code_verifier)
        else {
            return;
        };
        if matches!(self.token_response, Some(Ok(_))) {
            return;
        }

        let request = TokenRequest::new(&authorization.code, code_verifier);
        let response = request_token(&request).await;
        self.token = response
            .as_ref()
            .ok()
            .map(|response| response.access_token().to_string());
        self.token_request = Some(request);
        self.token_response = Some(response);
    }

    pub fn reset(&mut self) {
        *self = AuthProgress::default();
    }
}

fn check_callback(
    query: &CallbackQuery,
    expected_state: &State,
) -> Result<AuthorizationResponse, AuthError> {
    // Even an error has to answer this request, otherwise anyone could inject one.
    match &query.state {
        Some(state) if expected_state == &state.as_str() => {}
        _ => return Err(AuthError::StateMismatch),
    }

    let issuer = config::auth_server_url();
    if query.iss.as_deref() != Some(issuer.as_str()) {
        return Err(AuthError::IssuerMismatch {
            expected: issuer,
            actual: query.iss.clone(),
        });
    }

    if let Some(error) = &query.error {
        return Err(AuthError::AuthorizationDenied {
            error: error.clone(),
            error_description: query.error_description.clone(),
        });
    }

    let code = query.code.clone().ok_or(AuthError::MissingCode)?;
    Ok(AuthorizationResponse {
        code,
        state: expected_state.value().to_string(),
        iss: issuer,
        expires_in: query.expires_in.clone(),
    })
}
