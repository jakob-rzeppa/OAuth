mod request;

use axum::{
    http::header,
    response::{AppendHeaders, IntoResponse, Redirect, Response},
};

use crate::{
    domain::user_session::UserSession,
    login::{
        error_response::LoginErrorResponse,
        login_page::render_login_page,
        return_to::{ReturnToQuery, is_local_path},
        submit::request::LoginFormSubmitRequest,
    },
    persistence::users::authorize::{AuthorizeUserError, authorize_user},
    session,
};

#[axum::debug_handler]
pub async fn login_submit_endpoint(
    ReturnToQuery { return_to }: ReturnToQuery,
    request: LoginFormSubmitRequest,
) -> Result<Response, LoginErrorResponse> {
    let LoginFormSubmitRequest {
        user_name,
        password,
    } = request;

    // `return_to` comes from the query string, so it is as untrusted as the one the page was
    // served with.
    if !is_local_path(&return_to) {
        tracing::warn!("login submit rejected: return_to is not a local path");
        return Err(LoginErrorResponse::InvalidReturnTo);
    }

    let authorize_res = authorize_user(&user_name, &password).await;

    let user = match authorize_res {
        Ok(data) => data,
        Err(AuthorizeUserError::Internal(error)) => return Err(error.into()),
        Err(AuthorizeUserError::InvalidCredentials) => {
            // Expected user behaviour, so INFO. The user name is deliberately not logged.
            tracing::info!("login failed: invalid credentials");
            return Ok(render_login_page(
                return_to,
                Some(user_name),
                Some("Invalid username or password".to_string()),
            )
            .await?
            .into_response());
        }
    };

    // Allow temporary passwords for now.

    let session_cookie = session::issue(&UserSession::new(user.id)).await?;

    tracing::info!(user_id = %user.id, "user logged in");

    // 303, so the browser follows up with a GET even though this was a POST.
    Ok((
        AppendHeaders([(header::SET_COOKIE, session_cookie)]),
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
        config::Config,
        logging::testing::LogCapture,
        persistence::{
            user_sessions::save::save_session_mock,
            users::authorize::{AuthorizedUser, authorize_user_mock},
        },
        util::token::hash_token,
    };

    fn return_to(return_to: &str) -> ReturnToQuery {
        ReturnToQuery {
            return_to: return_to.to_string(),
        }
    }

    fn form() -> LoginFormSubmitRequest {
        LoginFormSubmitRequest {
            user_name: "alice".to_string(),
            password: "secret".to_string(),
        }
    }

    #[tokio::test]
    async fn redirects_to_return_to_and_sets_the_user_session_cookie() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        let user_id = Uuid::new_v4();
        let saved = Arc::new(Mutex::new(None));
        let saved_in_mock = saved.clone();

        authorize_user_mock().setup(move |_, _| {
            Ok(AuthorizedUser {
                id: user_id,
                has_temporary_password: false,
            })
        });
        Config::user_session_ttl_fake().setup(|| 1800);
        save_session_mock().setup(move |key, value, ttl_seconds| {
            *saved_in_mock.lock().unwrap() =
                Some((key.to_string(), value.to_string(), ttl_seconds));
            Ok(())
        });

        let response = login_submit_endpoint(return_to("/authorize?client_id=1"), form())
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
        assert_eq!(cookies.len(), 1, "{cookies:?}");

        let cookie = cookies
            .iter()
            .find(|cookie| cookie.starts_with("__Host-user_session="))
            .expect("expected a __Host-user_session cookie");
        let token = cookie
            .strip_prefix("__Host-user_session=")
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

        let (key, value, ttl_seconds) = saved.lock().unwrap().clone().unwrap();
        assert_eq!(key, format!("session:user:{}", hash_token(token)));
        assert_eq!(
            serde_json::from_str::<UserSession>(&value).unwrap(),
            UserSession::new(user_id)
        );
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
    async fn shows_the_login_page_again_when_the_credentials_are_invalid() {
        let capture = LogCapture::default();
        let _guard = capture.install();
        authorize_user_mock().setup(|_, _| Err(AuthorizeUserError::InvalidCredentials));
        save_session_mock().setup(|_, _, _| Ok(()));

        let response = login_submit_endpoint(return_to("/authorize"), form())
            .await
            .ok()
            .expect("expected a successful result");

        assert_eq!(response.status(), StatusCode::OK);

        let log = capture.contents();
        assert!(log.contains("INFO"), "{log}");
        assert!(log.contains("login failed"), "{log}");
        assert!(!log.contains("WARN"), "{log}");
        assert!(!log.contains("alice"), "{log}");
        assert!(!log.contains("secret"), "{log}");
    }
}
