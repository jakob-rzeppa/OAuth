use axum::{Router, routing::post};
use uuid::Uuid;

use crate::{
    domain::entity::authorization_code::request::{
        AuthorizationRequest,
        validate::{FatalValidationError, RedirectableValidationError, ValidationError},
    },
    par::{
        error_response::AuthorizePushErrorResponse, request::AuthorizePushRequest,
        response::AuthorizePushResponse,
    },
    persistence::{clients::find_by_id::find_client_by_id, pars::save::save_par},
};

mod error_response;
mod request;
mod response;

pub fn router() -> Router {
    Router::new().route("/par", post(authorize_push_endpoint))
}

const PAR_TTL_SECONDS: u64 = 180; // 3 minutes

pub async fn authorize_push_endpoint(
    AuthorizePushRequest {
        client_id,
        redirect_uri,
        response_type,
        scope,
        state,
        code_challenge,
        code_challenge_method,
    }: AuthorizePushRequest,
) -> Result<AuthorizePushResponse, AuthorizePushErrorResponse> {
    let Ok(client_id) = Uuid::parse_str(&client_id) else {
        return Err(AuthorizePushErrorResponse::InvalidClientId);
    };

    let client = find_client_by_id(&client_id).ok_or_else(|| {
        tracing::warn!(%client_id, "pushed authorization request for an unknown client");
        AuthorizePushErrorResponse::ClientNotFound
    })?;

    let request = AuthorizationRequest::new(
        client_id,
        redirect_uri,
        response_type,
        scope,
        state,
        code_challenge,
        code_challenge_method,
    );

    request
        .validate_against_client(&client)
        .map_err(|err| match err {
            ValidationError::Fatal { error } => match error {
                FatalValidationError::ClientIdMismatch => {
                    AuthorizePushErrorResponse::InternalServerError
                }
                FatalValidationError::InvalidRedirectUri => {
                    AuthorizePushErrorResponse::InvalidRedirectUri
                }
                FatalValidationError::InvalidState => AuthorizePushErrorResponse::InvalidState,
            },
            ValidationError::Redirectable { error, .. } => match error {
                RedirectableValidationError::InvalidResponseType => {
                    AuthorizePushErrorResponse::InvalidResponseType
                }
                RedirectableValidationError::InvalidScope => {
                    AuthorizePushErrorResponse::InvalidScope
                }
                RedirectableValidationError::InvalidCodeChallengeMethod => {
                    AuthorizePushErrorResponse::InvalidCodeChallengeMethod
                }
                RedirectableValidationError::InvalidCodeChallenge => {
                    AuthorizePushErrorResponse::InvalidCodeChallenge
                }
            },
        })?;

    let request_uri = generate_request_uri();

    save_par(&request_uri, request, PAR_TTL_SECONDS)
        .await
        .map_err(|_| AuthorizePushErrorResponse::DatabaseError)?;

    Ok(AuthorizePushResponse {
        request_uri,
        expires_in: PAR_TTL_SECONDS,
    })
}

const REQUEST_URI_PREFIX: &str = "urn:authorize:request_uri:";

/// Generate a unique request URI for the pushed authorization request.
///
/// Per RFC 9126, the `request_uri` must be hard to guess (a "https" or "urn" scheme
/// value containing a cryptographically random component). We use a UUIDv4, which is
/// generated from a CSPRNG, appended to a URN prefix.
#[fnmock::fakeable]
fn generate_request_uri() -> String {
    format!("{REQUEST_URI_PREFIX}{}", Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::entity::client::Client,
        persistence::{
            clients::find_by_id::find_client_by_id_fake,
            pars::save::{SaveParError, save_par_fake},
        },
    };

    fn make_client(id: Uuid) -> Client {
        Client::new(
            id,
            "Test Client".to_string(),
            vec!["https://example.com/callback".to_string()],
            vec!["read".to_string(), "write".to_string()],
        )
    }

    fn valid_request(client_id: Uuid) -> AuthorizePushRequest {
        AuthorizePushRequest {
            client_id: client_id.to_string(),
            redirect_uri: "https://example.com/callback".to_string(),
            response_type: "code".to_string(),
            scope: "read write".to_string(),
            state: "some-state".to_string(),
            code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            code_challenge_method: "S256".to_string(),
        }
    }

    #[tokio::test]
    async fn succeeds_and_returns_request_uri_with_ttl_for_a_valid_request() {
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);

        find_client_by_id_fake().setup(move |_| Some(client.clone()));
        save_par_fake().setup(move |request_uri, par, ttl_seconds| {
            assert!(request_uri.starts_with(REQUEST_URI_PREFIX));
            assert_eq!(ttl_seconds, PAR_TTL_SECONDS);
            assert_eq!(par.client_id(), &client_id);
            assert_eq!(par.redirect_uri(), "https://example.com/callback");
            assert_eq!(par.state(), "some-state");
            Ok(())
        });
        generate_request_uri_fake().setup(|| "urn:authorize:request_uri:test".to_string());

        let result = authorize_push_endpoint(valid_request(client_id)).await;

        let Ok(response) = result else {
            panic!("expected a successful result");
        };
        assert_eq!(response.request_uri, "urn:authorize:request_uri:test");
        assert_eq!(response.expires_in, PAR_TTL_SECONDS);
    }

    #[tokio::test]
    async fn fails_with_invalid_client_id_when_client_id_is_not_a_uuid() {
        let mut request = valid_request(Uuid::new_v4());
        request.client_id = "not-a-uuid".to_string();

        let result = authorize_push_endpoint(request).await;

        assert!(matches!(
            result,
            Err(AuthorizePushErrorResponse::InvalidClientId)
        ));
    }

    #[tokio::test]
    async fn fails_when_client_is_not_found() {
        find_client_by_id_fake().setup(|_| None);

        let result = authorize_push_endpoint(valid_request(Uuid::new_v4())).await;

        assert!(matches!(
            result,
            Err(AuthorizePushErrorResponse::ClientNotFound)
        ));
    }

    #[tokio::test]
    async fn fails_when_redirect_uri_is_not_registered() {
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);
        find_client_by_id_fake().setup(move |_| Some(client.clone()));

        let mut request = valid_request(client_id);
        request.redirect_uri = "https://evil.example.com/callback".to_string();
        let result = authorize_push_endpoint(request).await;

        assert!(matches!(
            result,
            Err(AuthorizePushErrorResponse::InvalidRedirectUri)
        ));
    }

    #[tokio::test]
    async fn fails_with_database_error_when_save_par_fails() {
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);

        find_client_by_id_fake().setup(move |_| Some(client.clone()));
        save_par_fake().setup(|_, _, _| Err(SaveParError::DatabaseError));
        generate_request_uri_fake().setup(|| "urn:authorize:request_uri:test".to_string());

        let result = authorize_push_endpoint(valid_request(client_id)).await;

        assert!(matches!(
            result,
            Err(AuthorizePushErrorResponse::DatabaseError)
        ));
    }
}
