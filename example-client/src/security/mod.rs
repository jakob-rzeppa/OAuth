use crate::{
    config,
    security::auth::{
        authorization::{AuthorizationResponse, CallbackQuery, authorization_url},
        check::Check,
        code_verifier::CodeVerifier,
        error::AuthError,
        nonce::Nonce,
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
    pub nonce: Option<Nonce>,
    pub code_verifier: Option<CodeVerifier>,

    pub par_request: Option<ParRequest>,
    pub par_response: Option<Result<ParResponse, AuthError>>,

    pub auth_redirect_uri: Option<String>,
    pub callback_query: Option<CallbackQuery>,
    /// The `state` and `iss` of the last callback against the expected values.
    pub callback_checks: Vec<Check>,
    pub auth_response: Option<Result<AuthorizationResponse, AuthError>>,

    pub token_request: Option<TokenRequest>,
    pub token_response: Option<Result<TokenResponse, AuthError>>,
    /// The ID token's `iss` and `nonce` against the expected values.
    pub token_checks: Vec<Check>,

    pub token: Option<String>,
    /// The ID token, if the response carried one and it passed the checks.
    pub id_token: Option<String>,
}

impl AuthProgress {
    /// Step 1: generates the state, the nonce and the PKCE verifier, locally.
    pub fn prepare(&mut self) {
        if self.state.is_some() {
            return;
        }
        self.state = Some(State::generate());
        self.nonce = Some(Nonce::generate());
        self.code_verifier = Some(CodeVerifier::generate());
    }

    /// Step 2: pushes the authorization request to the auth-server, and with its `request_uri`
    /// builds the URL the browser is sent to.
    pub async fn push_par(&mut self) {
        let (Some(state), Some(nonce), Some(code_verifier)) =
            (&self.state, &self.nonce, &self.code_verifier)
        else {
            return;
        };
        if matches!(self.par_response, Some(Ok(_))) {
            return;
        }

        let request = ParRequest::new(state, nonce, code_verifier);
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

        let (checks, response) = check_callback(&query, expected_state);
        self.callback_checks = checks;
        self.auth_response = Some(response);
        self.callback_query = Some(query);
    }

    /// Step 4: exchanges the code for tokens and checks the ID token's issuer and nonce.
    pub async fn exchange_code(&mut self) {
        let (Some(Ok(authorization)), Some(nonce), Some(code_verifier)) =
            (&self.auth_response, &self.nonce, &self.code_verifier)
        else {
            return;
        };
        if matches!(self.token_response, Some(Ok(_))) {
            return;
        }

        let request = TokenRequest::new(&authorization.code, code_verifier);
        let (checks, response) = match request_token(&request).await {
            Ok(response) => check_token_response(response, nonce),
            Err(error) => (Vec::new(), Err(error)),
        };
        self.token_checks = checks;
        self.id_token = response
            .as_ref()
            .ok()
            .and_then(|response| response.id_token().map(str::to_string));
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

/// Compares the callback's `state` and `iss` with the expected ones, both always, so both can be
/// shown. Only then looks at the code or the error.
fn check_callback(
    query: &CallbackQuery,
    expected_state: &State,
) -> (Vec<Check>, Result<AuthorizationResponse, AuthError>) {
    let state = Check::new("state", expected_state.value(), query.state.clone());
    let iss = Check::new("iss", config::auth_server_url(), query.iss.clone());
    let result = verify_callback(query, &state, &iss);
    (vec![state, iss], result)
}

fn verify_callback(
    query: &CallbackQuery,
    state: &Check,
    iss: &Check,
) -> Result<AuthorizationResponse, AuthError> {
    // Even an error has to answer this request, otherwise anyone could inject one.
    if !state.passed() {
        return Err(AuthError::StateMismatch);
    }
    if !iss.passed() {
        return Err(AuthError::IssuerMismatch {
            expected: iss.expected.clone(),
            actual: iss.actual.clone(),
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
        state: state.expected.clone(),
        iss: iss.expected.clone(),
    })
}

/// The `openid` scope is always requested, so the response must carry an ID token issued by the
/// auth-server the request went to, for this request's nonce. Both are always compared, so both
/// can be shown.
fn check_token_response(
    response: TokenResponse,
    expected_nonce: &Nonce,
) -> (Vec<Check>, Result<TokenResponse, AuthError>) {
    let checks = vec![
        ("iss", Check::new("id_token iss", config::auth_server_url(), response.id_token_claim("iss"))),
        ("nonce", Check::new("id_token nonce", expected_nonce.value(), response.id_token_claim("nonce"))),
    ];
    let result = match checks.iter().find(|(_, check)| !check.passed()) {
        None => Ok(response),
        Some((claim, check)) => Err(AuthError::IdTokenClaimMismatch {
            claim,
            expected: check.expected.clone(),
            actual: check.actual.clone(),
        }),
    };
    (checks.into_iter().map(|(_, check)| check).collect(), result)
}
