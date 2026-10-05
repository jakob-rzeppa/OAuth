mod request;

use axum::{
    http::header,
    response::{IntoResponse, Redirect, Response},
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
        cookie::{USER_SESSION_COOKIE, session_cookie},
        error_response::LoginErrorResponse,
        page::login_page,
        return_to::is_local_path,
        submit::request::{LoginFormSubmitRequest, LoginSessionToken},
    },
};

#[axum::debug_handler]
pub async fn login_submit_endpoint(
    LoginSessionToken(session_token): LoginSessionToken,
    request: LoginFormSubmitRequest,
) -> Result<Response, LoginErrorResponse> {
    let LoginFormSubmitRequest {
        return_to,
        csrf_token,
        user_name,
        password,
    } = request;

    // `return_to` is a form field, so it is as untrusted as the query it was copied from.
    if !is_local_path(&return_to) {
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
        return Err(LoginErrorResponse::MissingSessionCookie);
    };

    if csrf_token != login_session.csrf_token() {
        return Err(LoginErrorResponse::InvalidCsrfToken);
    }

    let authorize_res = authorize_user(&user_name, &password).await;

    let user = match authorize_res {
        Ok(data) => data,
        Err(AuthorizeUserError::ServerError) => {
            return Err(LoginErrorResponse::DatabaseError);
        }
        Err(AuthorizeUserError::InvalidCredentials) => {
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

    // 303, so the browser follows up with a GET even though this was a POST.
    Ok(([(header::SET_COOKIE, cookie)], Redirect::to(&return_to)).into_response())
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
        persistence::{
            login_session::{save::save_login_session_mock, take::take_login_session_fake},
            user_session::save::save_user_session_mock,
            users::authorize::{AuthorizedUser, authorize_user_mock},
        },
    };

    fn form(csrf_token: &str, return_to: &str) -> LoginFormSubmitRequest {
        LoginFormSubmitRequest {
            return_to: return_to.to_string(),
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
            form("csrf", "/authorize?client_id=1"),
        )
        .await
        .ok()
        .expect("expected a successful result");

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(header::LOCATION).unwrap(),
            "/authorize?client_id=1"
        );

        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
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
    }

    #[tokio::test]
    async fn fails_with_invalid_csrf_token_when_it_differs_from_the_login_session() {
        take_login_session_fake().setup(|_| Ok(Some(LoginSession::new("csrf".to_string()))));

        let result =
            login_submit_endpoint(login_session_token(), form("other", "/authorize")).await;

        assert!(matches!(result, Err(LoginErrorResponse::InvalidCsrfToken)));
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
            let result =
                login_submit_endpoint(login_session_token(), form("wrong-csrf", return_to)).await;

            assert!(
                matches!(result, Err(LoginErrorResponse::InvalidReturnTo)),
                "{return_to:?}"
            );
        }
    }

    #[tokio::test]
    async fn shows_the_login_page_again_when_the_credentials_are_invalid() {
        take_login_session_fake().setup(|_| Ok(Some(LoginSession::new("csrf".to_string()))));
        authorize_user_mock().setup(|_, _| Err(AuthorizeUserError::InvalidCredentials));
        login_session_ttl_fake().setup(|| 900);
        save_login_session_mock().setup(|_, _, _| Ok(()));

        let response = login_submit_endpoint(login_session_token(), form("csrf", "/authorize"))
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
    }
}
