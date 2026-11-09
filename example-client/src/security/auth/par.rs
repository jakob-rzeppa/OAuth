use crate::{config, security::auth::{error::AuthError}};

#[derive(serde::Serialize)]
pub struct ParRequest {
    client_id: String,
    redirect_uri: String,
    response_type: String,
    scope: String,
    state: String,
    code_challenge: String,
    code_challenge_method: String,
}


#[derive(serde::Deserialize)]
pub struct ParResponse {
    request_uri: String,
}

pub async fn register_par(request: &ParRequest) -> Result<String, AuthError> {
    let client = reqwest::Client::new();

    let response = client
        .post(config::auth_server_url() + "/par")
        .form(&request)
        .send()
        .await?;
    let response: ParResponse = AuthError::check(response).await?.json().await?;

    Ok(response.request_uri)
}
