mod request;

use axum::{
    http::header,
    response::{AppendHeaders, IntoResponse, Redirect, Response},
};

use crate::{
    config::user_session_ttl,
    domain::entity::user_session::UserSession,
    persistence::{
        login_session::take::{TakeLoginSessionError, take_login_session},
        user_session::save::{SaveUserSessionError, save_user_session},
        users::authorize::{AuthorizeUserError, authorize_user},
    },
    util::session::{create_session_token, hash_session_token},
    web::login::{
        cookie::{LOGIN_SESSION_COOKIE, USER_SESSION_COOKIE, expired_cookie, session_cookie},
        error_response::LoginErrorResponse,
        page::login_page,
        return_to::{ReturnToQuery, is_local_path},
        submit::request::{LoginFormSubmitRequest, LoginSessionToken},
    },
};

#[axum::debug_handler]
pub async fn login_submit_endpoint(
    LoginSessionToken(session_token): LoginSessionToken,
    ReturnToQuery { return_to }: ReturnToQuery,
    request: LoginFormSubmitRequest,
) -> Result<Response, LoginErrorResponse> {
    let LoginFormSubmitRequest {
        csrf_token,
        user_name,
        password,
    } = request;

    // `return_to` comes from the query string, so it is as untrusted as the one the page was
    // served with.
    if !is_local_path(&return_to) {
        tracing::warn!("login submit rejected: return_to is not a local path");
        return Err(LoginErrorResponse::InvalidReturnTo);
    }

    let login_session_token_hash = hash_session_token(&session_token);
    let login_session = take_login_session(&login_session_token_hash)
        .await
        .map_err(|e| match e {
            TakeLoginSessionError::DatabaseError | TakeLoginSessionError::InvalidData => {
                LoginErrorResponse::DatabaseError
            }
        })?;

    let Some(login_session) = login_session else {
        tracing::warn!("login submit rejected: no valid login session");
        return Err(LoginErrorResponse::MissingSessionCookie);
    };

    if csrf_token != login_session.csrf_token() {
        tracing::warn!("login submit rejected: invalid CSRF token");
        return Err(LoginErrorResponse::InvalidCsrfToken);
    }

    let authorize_res = authorize_user(&user_name, &password).await;

    let user = match authorize_res {
        Ok(data) => data,
        Err(AuthorizeUserError::ServerError) => {
            return Err(LoginErrorResponse::DatabaseError);
        }
        Err(AuthorizeUserError::InvalidCredentials) => {
            // Expected user behaviour, so INFO. The user name is deliberately not logged.
            tracing::info!("login failed: invalid credentials");
            return Ok(login_page(
                return_to,
                Some(user_name),
                Some("Invalid username or password".to_string()),
            )
            .await?
            .into_response());
        }
    };

    // Allow temporary passwords for now.

    let session_token = create_session_token();
    let session_ttl = user_session_ttl();
    let user_session_entity = UserSession::new(user.id);

    save_user_session(
        &hash_session_token(&session_token),
        user_session_entity,
        session_ttl,
    )
    .await
    .map_err(|e| match e {
        SaveUserSessionError::DatabaseError => LoginErrorResponse::DatabaseError,
        SaveUserSessionError::SerializationError => LoginErrorResponse::DatabaseError,
    })?;

    let cookie = session_cookie(USER_SESSION_COOKIE, &session_token, session_ttl)
        .ok_or(LoginErrorResponse::DatabaseError)?;

    // The login session was consumed above, so its cookie is of no use any more.
    let clear_login_session_cookie =
        expired_cookie(LOGIN_SESSION_COOKIE).ok_or(LoginErrorResponse::DatabaseError)?;

    tracing::info!(user_id = %user.id, "user logged in");

    // 303, so the browser follows up with a GET even though this was a POST.
    Ok((
        AppendHeaders([
            (header::SET_COOKIE, cookie),
            (header::SET_COOKIE, clear_login_session_cookie),
        ]),
        Redirect::to(&return_to),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::http::StatusCode;
    use uuid::Uuid;

    use super::*;
    use crate::{
        config::{login_session_ttl_fake, user_session_ttl_fake},
        domain::entity::login_session::LoginSession,
        logging::testing::LogCapture,
        persistence::{
            login_session::{save::save_login_session_mock, take::take_login_session_fake},
            user_session::save::save_user_session_mock,
            users::authorize::{AuthorizedUser, authorize_user_mock},
        },
    };

    fn return_to(return_to: &str) -> ReturnToQuery {
        ReturnToQuery {
            return_to: return_to.to_string(),
        }
    }

    fn form(csrf_token: &str) -> LoginFormSubmitRequest {
        LoginFormSubmitRequest {
            csrf_token: csrf_token.to_string(),
            user_name: "alice".to_string(),
            password: "secret".to_string(),
        }
    }

    fn login_session_token() -> LoginSessionToken {
        LoginSessionToken("login-token".to_string())
    }

    #[tokio::test]
    async fn redirects_to_return_to_and_sets_the_user_session_cookie() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let user_id = Uuid::new_v4();
        let saved = Arc::new(Mutex::new(None));
        let saved_in_mock = saved.clone();

        take_login_session_fake().setup(|hash| {
            assert_eq!(hash, hash_session_token("login-token"));
            Ok(Some(LoginSession::new("csrf".to_string())))
        });
        authorize_user_mock().setup(move |_, _| {
            Ok(AuthorizedUser {
                id: user_id,
                has_temporary_password: false,
            })
        });
        user_session_ttl_fake().setup(|| 1800);
        save_user_session_mock().setup(move |hash, session, ttl_seconds| {
            *saved_in_mock.lock().unwrap() = Some((hash.to_string(), session, ttl_seconds));
            Ok(())
        });

        let response = login_submit_endpoint(
            login_session_token(),
            return_to("/authorize?client_id=1"),
            form("csrf"),
        )
        .await
        .ok()
        .expect("expected a successful result");

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(header::LOCATION).unwrap(),
            "/authorize?client_id=1"
        );

        let cookies: Vec<&str> = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect();
        assert_eq!(cookies.len(), 2, "{cookies:?}");

        // The consumed login session cookie is deleted.
        assert!(
            cookies.contains(&"login_session=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax"),
            "{cookies:?}"
        );

        let cookie = cookies
            .iter()
            .find(|cookie| cookie.starts_with("user_session="))
            .expect("expected a user_session cookie");
        let token = cookie
            .strip_prefix("user_session=")
            .unwrap()
            .split(';')
            .next()
            .unwrap();
        for attribute in [
            "Max-Age=1800",
            "Path=/",
            "HttpOnly",
            "Secure",
            "SameSite=Lax",
        ] {
            assert!(cookie.contains(attribute), "{cookie}");
        }

        let (hash, session, ttl_seconds) = saved.lock().unwrap().clone().unwrap();
        assert_eq!(hash, hash_session_token(token));
        assert_eq!(session, UserSession::new(user_id));
        assert_eq!(ttl_seconds, 1800);

        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("user logged in"), "{log}");
        assert!(log.contains(&user_id.to_string()), "{log}");
        assert!(!log.contains(token), "{log}");
        assert!(!log.contains("alice"), "{log}");
        assert!(!log.contains("secret"), "{log}");
    }

    #[tokio::test]
    async fn fails_with_invalid_csrf_token_when_it_differs_from_the_login_session() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        take_login_session_fake().setup(|_| Ok(Some(LoginSession::new("csrf".to_string()))));

        let result = login_submit_endpoint(
            login_session_token(),
            return_to("/authorize"),
            form("other"),
        )
        .await;

        assert!(matches!(result, Err(LoginErrorResponse::InvalidCsrfToken)));

        let log = capture.contents();
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("CSRF"), "{log}");
        assert!(!log.contains("other"), "{log}");
    }

    #[tokio::test]
    async fn fails_with_invalid_return_to_before_touching_the_login_session() {
        // Nothing is faked here: the login session is neither taken nor checked, so a bad
        // `return_to` can't burn the session, and the error is not hidden by a CSRF or Redis error.
        for return_to in [
            "",
            "https://evil.example",
            "//evil.example",
            "/\\evil.example",
            "authorize",
            "/ok\r\nSet-Cookie: a=b",
        ] {
            let result = login_submit_endpoint(
                login_session_token(),
                self::return_to(return_to),
                form("wrong-csrf"),
            )
            .await;

            assert!(
                matches!(result, Err(LoginErrorResponse::InvalidReturnTo)),
                "{return_to:?}"
            );
        }
    }

    #[tokio::test]
    async fn shows_the_login_page_again_when_the_credentials_are_invalid() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        take_login_session_fake().setup(|_| Ok(Some(LoginSession::new("csrf".to_string()))));
        authorize_user_mock().setup(|_, _| Err(AuthorizeUserError::InvalidCredentials));
        login_session_ttl_fake().setup(|| 900);
        save_login_session_mock().setup(|_, _, _| Ok(()));

        let response =
            login_submit_endpoint(login_session_token(), return_to("/authorize"), form("csrf"))
                .await
                .ok()
                .expect("expected a successful result");

        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cookie.starts_with("login_session="), "{cookie}");

        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("login failed"), "{log}");
        assert!(!log.contains("WARN"), "{log}");
        assert!(!log.contains("alice"), "{log}");
        assert!(!log.contains("secret"), "{log}");
    }
}
