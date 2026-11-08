use crate::{config, security::auth::{code_verifier::CodeVerifier, state::State}};

pub enum ParError {
    RequestError(reqwest::Error),
}

pub async fn register_par(state: &State, code_verifier: &CodeVerifier) -> Result<String, ParError> {
    let client = reqwest::Client::new();

    #[derive(serde::Serialize)]
    struct Request<'a> {
        client_id: &'a str,
        redirect_uri: &'a str,
        response_type: &'a str,
        scope: &'a str,
        state: &'a str,
        code_challenge: &'a str,
        code_challenge_method: &'a str,
    }

    let request = Request {
        client_id: &config::client_id(),
        redirect_uri: &config::home_url(),
        response_type: "code",
        scope: "openid profile",
        state: state.value(),
        code_challenge: &code_verifier.code_challenge(),
        code_challenge_method: code_verifier.code_challenge_method(),
    };

    #[derive(serde::Deserialize)]
    struct Response {
        request_uri: String,
    }

    let response: Response = client
        .post(config::auth_server_url() + "/par")
        .form(&request)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(ParError::RequestError)?
        .json()
        .await
        .map_err(ParError::RequestError)?;

    Ok(response.request_uri)
}
