use crate::{
    authorize::{
        error_page::AuthorizeErrorPage,
        submit::{
            error_response::{AuthorizeSubmitErrorResponse, AuthorizeSubmitRedirectErrorResponse},
            request::AuthorizeSubmitRequest,
            response::AuthorizeSubmitResponse,
        },
    },
    config::{authorization_code_ttl, iss},
    domain::entity::authorization_code::{
        code::AuthorizationCode, request::validate::ValidatedAuthorizationRequest,
    },
    persistence::{
        authorization_codes::save::save_authorization_code, clients::find_by_id::find_client_by_id,
        pars::take::take_par,
    },
    security::require_session::require_user_session,
    security::session::UserSessionToken,
    util::token::random_token,
};

mod error_response;
mod request;
mod response;

#[axum::debug_handler]
pub async fn authorize_submit_endpoint(
    session_token: Option<UserSessionToken>,
    AuthorizeSubmitRequest {
        request_uri,
        client_id,
        decision,
    }: AuthorizeSubmitRequest,
) -> Result<AuthorizeSubmitResponse, AuthorizeSubmitErrorResponse> {
    // Make sure the user is logged in and has a valid session.
    // If not, return a redirect to the login page.
    let session = require_user_session(session_token, client_id, &request_uri).await?;
    let user_id = session.user_id();

    // Consume the pushed authorization request always, even if the user denied the request or the CSRF token is invalid.
    // Every invalid request is treated as a attack and the request is consumed.
    let request = take_par(&request_uri).await?.ok_or_else(|| {
        tracing::warn!(%client_id, "authorization submitted for an unknown, expired or already used request_uri");
        AuthorizeErrorPage::RequestNotFound
    })?;

    let client = find_client_by_id(&client_id).ok_or_else(|| {
        tracing::warn!(%client_id, "authorization submitted for an unknown client");
        AuthorizeErrorPage::ClientNotFound
    })?;

    let ValidatedAuthorizationRequest {
        client_id,
        redirect_uri,
        scope,
        state,
        code_challenge,
        code_challenge_method,
    } = request
        .validate_and_unpack(&client)
        .map_err(AuthorizeSubmitErrorResponse::from)?;

    // We only check the decision afer a full validation run, so we only return a redirect error if the request, client etc. are valid.
    if decision == false {
        tracing::info!(%client_id, "consent denied");
        return Err(AuthorizeSubmitErrorResponse::Redirect {
            error: AuthorizeSubmitRedirectErrorResponse::AccessDenied,
            redirect_uri,
            state,
        });
    }

    // With the request validated and the user's approval, we can now generate an authorization code and return it to the client.

    let code = random_token();
    let issued_scope = scope.clone();

    let authorization_code = AuthorizationCode::new(
        code.clone(),
        client_id,
        scope,
        user_id,
        code_challenge,
        code_challenge_method,
    );

    let ttl_seconds = authorization_code_ttl();

    save_authorization_code(authorization_code, ttl_seconds)
        .await
        .map_err(|_| {
            AuthorizeSubmitErrorResponse::server_error_redirect(redirect_uri.clone(), state.clone())
        })?;

    tracing::info!(%client_id, scope = %issued_scope, "authorization code issued");

    Ok(AuthorizeSubmitResponse {
        code,
        redirect_uri,
        state,
        expires_in: ttl_seconds,
        iss: iss().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{authorization_code_ttl_fake, iss_fake, user_session_ttl_fake},
        domain::entity::{
            authorization_code::request::AuthorizationRequest, client::Client,
            user_session::UserSession,
        },
        logging::testing::LogCapture,
        persistence::{
            authorization_codes::save::{SaveAuthorizationCodeError, save_authorization_code_mock},
            clients::find_by_id::find_client_by_id_fake,
            pars::take::{TakeParError, take_par_mock},
            user_session::access::{AccessUserSessionError, access_user_session_fake},
        },
    };
    use fnmock::predicate;
    use uuid::Uuid;

    use crate::util::token::random_token_fake;

    const CODE_TTL_SECONDS: u64 = 300;

    fn session_token() -> Option<UserSessionToken> {
        Some(UserSessionToken::new("session-token".to_string()))
    }

    /// Fakes a valid user session for the returned user.
    fn fake_valid_session() -> Uuid {
        let user_id = Uuid::new_v4();
        user_session_ttl_fake().setup(|| 1800);
        access_user_session_fake().setup(move |_, _| Ok(Some(UserSession::new(user_id))));
        user_id
    }

    fn make_client(id: Uuid) -> Client {
        Client::new(
            id,
            "Test Client".to_string(),
            vec!["https://example.com/callback".to_string()],
            vec!["read".to_string(), "write".to_string()],
        )
    }

    fn make_request_with(client_id: Uuid, redirect_uri: &str, scope: &str) -> AuthorizationRequest {
        AuthorizationRequest::new(
            client_id,
            redirect_uri.to_string(),
            "code".to_string(),
            scope.to_string(),
            "some-state".to_string(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            "S256".to_string(),
        )
    }

    fn make_request(client_id: Uuid) -> AuthorizationRequest {
        make_request_with(client_id, "https://example.com/callback", "read write")
    }

    fn submit_request(client_id: Uuid, request_uri: &str) -> AuthorizeSubmitRequest {
        AuthorizeSubmitRequest {
            request_uri: request_uri.to_string(),
            client_id,
            decision: true,
        }
    }

    #[tokio::test]
    async fn succeeds_and_returns_code_with_ttl_for_a_valid_request() {
        let user_id = fake_valid_session();
        let capture = LogCapture::default();
        let _guard = capture.install();
        let client_id = Uuid::new_v4();
        let request_uri = "urn:authorize:request_uri:test";
        let client = make_client(client_id);
        let request = make_request(client_id);

        let take_mock = take_par_mock();
        take_mock.setup(move |_| Ok(Some(request.clone())));
        take_mock.expect(predicate::eq(request_uri)).once();
        find_client_by_id_fake().setup(move |_| Some(client.clone()));
        random_token_fake().setup(|| "test-code".to_string());
        authorization_code_ttl_fake().setup(|| CODE_TTL_SECONDS);
        iss_fake().setup(|| "test-issuer");
        let save_mock = save_authorization_code_mock();
        save_mock.setup(|_, _| Ok(()));
        save_mock
            .expectf(move |code: &AuthorizationCode, ttl_seconds: &u64| {
                code.code() == "test-code"
                    && code.client_id() == &client_id
                    && code.sub() == &user_id
                    && code.scope() == "read write"
                    && code.code_challenge() == "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
                    && code.code_challenge_method() == "S256"
                    && *ttl_seconds == CODE_TTL_SECONDS
            })
            .once();

        let result =
            authorize_submit_endpoint(session_token(), submit_request(client_id, request_uri))
                .await;

        let Ok(response) = result else {
            panic!("expected a successful result");
        };
        assert_eq!(response.code, "test-code");
        assert_eq!(response.redirect_uri, "https://example.com/callback");
        assert_eq!(response.state, "some-state");
        assert_eq!(response.expires_in, CODE_TTL_SECONDS);
        assert_eq!(response.iss, "test-issuer");

        take_mock.assert();
        save_mock.assert();

        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("authorization code issued"), "{log}");
        assert!(log.contains(&client_id.to_string()), "{log}");
        assert!(log.contains("read write"), "{log}");
        assert!(!log.contains("test-code"), "{log}");
        assert!(
            !log.contains("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"),
            "{log}"
        );
        assert!(!log.contains(request_uri), "{log}");
    }

    #[tokio::test]
    async fn fails_with_server_error_when_take_par_fails() {
        fake_valid_session();
        let request_uri = "urn:authorize:request_uri:test";

        let take_mock = take_par_mock();
        take_mock.setup(|_| Err(TakeParError::DatabaseError));
        take_mock.expect(predicate::eq(request_uri)).once();
        let save_mock = save_authorization_code_mock();
        save_mock.expect_never();

        let result =
            authorize_submit_endpoint(session_token(), submit_request(Uuid::new_v4(), request_uri))
                .await;

        assert!(matches!(
            result,
            Err(AuthorizeSubmitErrorResponse::Page {
                error: AuthorizeErrorPage::ServerError
            })
        ));

        take_mock.assert();
        save_mock.assert();
    }

    #[tokio::test]
    async fn fails_when_pushed_request_is_not_found() {
        fake_valid_session();
        let capture = LogCapture::default();
        let _guard = capture.install();
        let request_uri = "urn:authorize:request_uri:missing";

        let take_mock = take_par_mock();
        take_mock.setup(|_| Ok(None));
        take_mock.expect(predicate::eq(request_uri)).once();
        let save_mock = save_authorization_code_mock();
        save_mock.expect_never();

        let result =
            authorize_submit_endpoint(session_token(), submit_request(Uuid::new_v4(), request_uri))
                .await;

        assert!(matches!(
            result,
            Err(AuthorizeSubmitErrorResponse::Page {
                error: AuthorizeErrorPage::RequestNotFound
            })
        ));

        take_mock.assert();
        save_mock.assert();

        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(
            log.contains("unknown, expired or already used request_uri"),
            "{log}"
        );
        assert!(!log.contains(request_uri), "{log}");
    }

    #[tokio::test]
    async fn fails_when_client_is_not_found() {
        fake_valid_session();
        let client_id = Uuid::new_v4();
        let request_uri = "urn:authorize:request_uri:test";
        let request = make_request(client_id);

        let take_mock = take_par_mock();
        take_mock.setup(move |_| Ok(Some(request.clone())));
        take_mock.expect(predicate::eq(request_uri)).once();
        find_client_by_id_fake().setup(|_| None);
        let save_mock = save_authorization_code_mock();
        save_mock.expect_never();

        let result =
            authorize_submit_endpoint(session_token(), submit_request(client_id, request_uri))
                .await;

        assert!(matches!(
            result,
            Err(AuthorizeSubmitErrorResponse::Page {
                error: AuthorizeErrorPage::ClientNotFound
            })
        ));

        take_mock.assert();
        save_mock.assert();
    }

    #[tokio::test]
    async fn fails_with_page_error_when_redirect_uri_is_not_registered() {
        fake_valid_session();
        let capture = LogCapture::default();
        let _guard = capture.install();
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);
        let request_uri = "urn:authorize:request_uri:test";
        let request = make_request_with(client_id, "https://evil.example.com/callback", "read");

        let take_mock = take_par_mock();
        take_mock.setup(move |_| Ok(Some(request.clone())));
        take_mock.expect(predicate::eq(request_uri)).once();
        find_client_by_id_fake().setup(move |_| Some(client.clone()));
        let save_mock = save_authorization_code_mock();
        save_mock.expect_never();

        let result =
            authorize_submit_endpoint(session_token(), submit_request(client_id, request_uri))
                .await;

        assert!(matches!(
            result,
            Err(AuthorizeSubmitErrorResponse::Page {
                error: AuthorizeErrorPage::InvalidRedirectUri
            })
        ));

        take_mock.assert();
        save_mock.assert();

        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("redirect_uri is not registered"), "{log}");
        assert!(!log.contains("evil.example.com"), "{log}");
    }

    #[tokio::test]
    async fn fails_with_redirect_error_when_scope_is_not_allowed() {
        fake_valid_session();
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);
        let request_uri = "urn:authorize:request_uri:test";
        let request = make_request_with(client_id, "https://example.com/callback", "delete");

        let take_mock = take_par_mock();
        take_mock.setup(move |_| Ok(Some(request.clone())));
        take_mock.expect(predicate::eq(request_uri)).once();
        find_client_by_id_fake().setup(move |_| Some(client.clone()));
        let save_mock = save_authorization_code_mock();
        save_mock.expect_never();

        let result =
            authorize_submit_endpoint(session_token(), submit_request(client_id, request_uri))
                .await;

        let Err(AuthorizeSubmitErrorResponse::Redirect {
            error: AuthorizeSubmitRedirectErrorResponse::InvalidScope,
            redirect_uri,
            state,
        }) = result
        else {
            panic!("expected an invalid_scope redirect error");
        };
        assert_eq!(redirect_uri, "https://example.com/callback");
        assert_eq!(state, "some-state");

        take_mock.assert();
        save_mock.assert();
    }

    #[tokio::test]
    async fn fails_with_redirect_server_error_when_save_authorization_code_fails() {
        fake_valid_session();
        let capture = LogCapture::default();
        let _guard = capture.install();
        let client_id = Uuid::new_v4();
        let request_uri = "urn:authorize:request_uri:test";
        let client = make_client(client_id);
        let request = make_request(client_id);

        let take_mock = take_par_mock();
        take_mock.setup(move |_| Ok(Some(request.clone())));
        take_mock.expect(predicate::eq(request_uri)).once();
        find_client_by_id_fake().setup(move |_| Some(client.clone()));
        random_token_fake().setup(|| "test-code".to_string());
        authorization_code_ttl_fake().setup(|| CODE_TTL_SECONDS);
        let save_mock = save_authorization_code_mock();
        save_mock.setup(|_, _| Err(SaveAuthorizationCodeError::DatabaseError));
        save_mock.expect_once();

        let result =
            authorize_submit_endpoint(session_token(), submit_request(client_id, request_uri))
                .await;

        let Err(AuthorizeSubmitErrorResponse::Redirect {
            error: AuthorizeSubmitRedirectErrorResponse::ServerError,
            redirect_uri,
            state,
        }) = result
        else {
            panic!("expected a server_error redirect error");
        };
        assert_eq!(redirect_uri, "https://example.com/callback");
        assert_eq!(state, "some-state");

        take_mock.assert();
        save_mock.assert();

        assert!(!capture.contents().contains("authorization code issued"));
    }

    #[tokio::test]
    async fn consumes_par_and_redirects_with_access_denied_when_user_denies() {
        fake_valid_session();
        let capture = LogCapture::default();
        let _guard = capture.install();
        let client_id = Uuid::new_v4();
        let request_uri = "urn:authorize:request_uri:test";
        let client = make_client(client_id);
        let request = make_request(client_id);

        let take_mock = take_par_mock();
        take_mock.setup(move |_| Ok(Some(request.clone())));
        take_mock.expect(predicate::eq(request_uri)).once();
        find_client_by_id_fake().setup(move |_| Some(client.clone()));
        let save_mock = save_authorization_code_mock();
        save_mock.expect_never();

        let result = authorize_submit_endpoint(
            session_token(),
            AuthorizeSubmitRequest {
                decision: false,
                ..submit_request(client_id, request_uri)
            },
        )
        .await;

        let Err(AuthorizeSubmitErrorResponse::Redirect {
            error: AuthorizeSubmitRedirectErrorResponse::AccessDenied,
            redirect_uri,
            state,
        }) = result
        else {
            panic!("expected an access_denied redirect error");
        };
        assert_eq!(redirect_uri, "https://example.com/callback");
        assert_eq!(state, "some-state");

        take_mock.assert();
        save_mock.assert();

        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("consent denied"), "{log}");
        assert!(log.contains(&client_id.to_string()), "{log}");
    }

    #[tokio::test]
    async fn redirects_to_login_without_a_session_token() {
        let take_mock = take_par_mock();
        take_mock.expect_never();
        let save_mock = save_authorization_code_mock();
        save_mock.expect_never();

        let result = authorize_submit_endpoint(
            None,
            submit_request(Uuid::new_v4(), "urn:authorize:request_uri:test"),
        )
        .await;

        assert!(matches!(
            result,
            Err(AuthorizeSubmitErrorResponse::Page {
                error: AuthorizeErrorPage::LoginRequired(_)
            })
        ));
        take_mock.assert();
        save_mock.assert();
    }

    #[tokio::test]
    async fn redirects_to_login_without_consuming_the_par_when_the_session_is_expired() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let client_id = Uuid::new_v4();
        let request_uri = "urn:authorize:request_uri:test";

        user_session_ttl_fake().setup(|| 1800);
        access_user_session_fake().setup(|_, _| Ok(None));
        let take_mock = take_par_mock();
        take_mock.expect_never();
        let save_mock = save_authorization_code_mock();
        save_mock.expect_never();

        let result =
            authorize_submit_endpoint(session_token(), submit_request(client_id, request_uri))
                .await;

        let Err(AuthorizeSubmitErrorResponse::Page {
            error: AuthorizeErrorPage::LoginRequired(redirect),
        }) = result
        else {
            panic!("expected a login required error");
        };
        assert_eq!(redirect.client_id, client_id);
        assert_eq!(redirect.request_uri, request_uri);
        take_mock.assert();
        save_mock.assert();

        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(
            log.contains("Expired or invalid user session token"),
            "{log}"
        );
        assert!(!log.contains("session-token"), "{log}");
    }

    #[tokio::test]
    async fn fails_with_server_error_when_access_user_session_fails() {
        user_session_ttl_fake().setup(|| 1800);
        access_user_session_fake().setup(|_, _| Err(AccessUserSessionError::DatabaseError));
        let take_mock = take_par_mock();
        take_mock.expect_never();
        let save_mock = save_authorization_code_mock();
        save_mock.expect_never();

        let result = authorize_submit_endpoint(
            session_token(),
            submit_request(Uuid::new_v4(), "urn:authorize:request_uri:test"),
        )
        .await;

        assert!(matches!(
            result,
            Err(AuthorizeSubmitErrorResponse::Page {
                error: AuthorizeErrorPage::ServerError
            })
        ));
        take_mock.assert();
        save_mock.assert();
    }
}
