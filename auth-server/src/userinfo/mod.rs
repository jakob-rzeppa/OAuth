use axum::{
    Router,
    http::{HeaderMap, header},
    middleware::map_response,
    routing::get,
};

use crate::{
    domain::authorization_code::openid::{has_openid_scope, has_profile_scope},
    persistence::{
        access_tokens::find_by_token_hash::find_access_token_by_token_hash,
        users::find_by_id::find_user_by_id,
    },
    security::cache_control::no_store,
    userinfo::{error_response::UserInfoErrorResponse, response::UserInfoResponse},
    util::token::hash_token,
};

mod error_response;
mod response;

/// OpenID Connect requires both GET and POST.
pub fn router() -> Router {
    Router::new()
        .route("/userinfo", get(userinfo_endpoint).post(userinfo_endpoint))
        .layer(map_response(no_store))
}

/// Returns the claims about the user the access token was issued for, filtered by its scope.
#[axum::debug_handler]
pub async fn userinfo_endpoint(
    headers: HeaderMap,
) -> Result<UserInfoResponse, UserInfoErrorResponse> {
    let token = bearer_token(&headers).ok_or(UserInfoErrorResponse::MissingToken)?;

    let Some(access_token) = find_access_token_by_token_hash(&hash_token(token)).await? else {
        return Err(UserInfoErrorResponse::InvalidToken);
    };

    if access_token.exp().timestamp() < chrono::Utc::now().timestamp() {
        return Err(UserInfoErrorResponse::InvalidToken);
    }

    // A token without a user (client credentials) can't carry `openid`, but it has no claims either.
    let (true, Some(sub)) = (has_openid_scope(access_token.scope()), access_token.sub()) else {
        return Err(UserInfoErrorResponse::InsufficientScope);
    };

    let mut response = UserInfoResponse {
        sub: *sub,
        name: None,
        preferred_username: None,
    };

    if has_profile_scope(access_token.scope()) {
        let Some(user) = find_user_by_id(*sub).await? else {
            tracing::warn!(%sub, "userinfo requested for a user that no longer exists");
            return Err(UserInfoErrorResponse::InvalidToken);
        };

        response.name = Some(user.display_name);
        response.preferred_username = Some(user.user_name);
    }

    Ok(response)
}

/// The token of an `Authorization: Bearer <token>` header (RFC 6750, section 2.1). The scheme is
/// case-insensitive.
fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();

    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::access_token::AccessToken,
        error::InternalError,
        persistence::{
            access_tokens::find_by_token_hash::find_access_token_by_token_hash_mock,
            users::find_by_id::{User, find_user_by_id_mock},
        },
    };
    use axum::{http::HeaderValue, response::IntoResponse};
    use fnmock::predicate;
    use uuid::Uuid;

    fn headers_with_token(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        headers
    }

    fn make_access_token(sub: Option<Uuid>, scope: &str, expires_in: i64) -> AccessToken {
        let now = chrono::Utc::now();
        AccessToken::new(
            hash_token("the-token"),
            "bearer".to_string(),
            Uuid::new_v4(),
            sub,
            now,
            now + chrono::Duration::seconds(expires_in),
            scope.to_string(),
        )
    }

    /// Fakes the stored access token for `the-token`.
    fn fake_access_token(sub: Option<Uuid>, scope: &'static str, expires_in: i64) {
        find_access_token_by_token_hash_mock()
            .setup(move |_| Ok(Some(make_access_token(sub, scope, expires_in))));
    }

    async fn body(response: UserInfoResponse) -> serde_json::Value {
        let body = axum::body::to_bytes(response.into_response().into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn returns_only_the_sub_without_the_profile_scope() {
        let sub = Uuid::new_v4();
        fake_access_token(Some(sub), "openid read", 3600);
        let user_mock = find_user_by_id_mock();
        user_mock.expect_never();

        let response = userinfo_endpoint(headers_with_token("the-token"))
            .await
            .unwrap();

        assert_eq!(body(response).await, serde_json::json!({ "sub": sub }));
        user_mock.assert();
    }

    #[tokio::test]
    async fn adds_the_profile_claims_with_the_profile_scope() {
        let sub = Uuid::new_v4();
        fake_access_token(Some(sub), "openid profile", 3600);
        let user_mock = find_user_by_id_mock();
        user_mock.setup(|id| {
            Ok(Some(User {
                id,
                user_name: "alice".to_string(),
                display_name: "Alice Liddell".to_string(),
            }))
        });
        user_mock.expect(predicate::eq(sub)).once();

        let response = userinfo_endpoint(headers_with_token("the-token"))
            .await
            .unwrap();

        assert_eq!(
            body(response).await,
            serde_json::json!({
                "sub": sub,
                "name": "Alice Liddell",
                "preferred_username": "alice",
            })
        );
        user_mock.assert();
    }

    #[tokio::test]
    async fn looks_the_token_up_by_its_hash() {
        let mock = find_access_token_by_token_hash_mock();
        mock.setup(|_| Ok(None));
        mock.expect(predicate::eq(hash_token("the-token"))).once();

        let _ = userinfo_endpoint(headers_with_token("the-token")).await;

        mock.assert();
    }

    #[tokio::test]
    async fn rejects_a_request_without_a_token() {
        let result = userinfo_endpoint(HeaderMap::new()).await;

        assert!(matches!(result, Err(UserInfoErrorResponse::MissingToken)));
    }

    #[tokio::test]
    async fn rejects_an_unknown_token_as_invalid() {
        find_access_token_by_token_hash_mock().setup(|_| Ok(None));

        let result = userinfo_endpoint(headers_with_token("the-token")).await;

        assert!(matches!(result, Err(UserInfoErrorResponse::InvalidToken)));
    }

    #[tokio::test]
    async fn rejects_an_expired_token_as_invalid() {
        fake_access_token(Some(Uuid::new_v4()), "openid", -1);

        let result = userinfo_endpoint(headers_with_token("the-token")).await;

        assert!(matches!(result, Err(UserInfoErrorResponse::InvalidToken)));
    }

    #[tokio::test]
    async fn rejects_a_token_without_the_openid_scope_as_insufficient_scope() {
        fake_access_token(Some(Uuid::new_v4()), "read write", 3600);

        let result = userinfo_endpoint(headers_with_token("the-token")).await;

        assert!(matches!(
            result,
            Err(UserInfoErrorResponse::InsufficientScope)
        ));
    }

    #[tokio::test]
    async fn rejects_a_token_without_a_user_as_insufficient_scope() {
        fake_access_token(None, "openid", 3600);

        let result = userinfo_endpoint(headers_with_token("the-token")).await;

        assert!(matches!(
            result,
            Err(UserInfoErrorResponse::InsufficientScope)
        ));
    }

    #[tokio::test]
    async fn rejects_the_token_when_its_user_no_longer_exists() {
        fake_access_token(Some(Uuid::new_v4()), "openid profile", 3600);
        find_user_by_id_mock().setup(|_| Ok(None));

        let result = userinfo_endpoint(headers_with_token("the-token")).await;

        assert!(matches!(result, Err(UserInfoErrorResponse::InvalidToken)));
    }

    #[tokio::test]
    async fn fails_with_a_server_error_when_the_lookup_fails() {
        find_access_token_by_token_hash_mock()
            .setup(|_| Err(InternalError::Invariant("postgres is down")));

        let result = userinfo_endpoint(headers_with_token("the-token")).await;

        assert!(matches!(result, Err(UserInfoErrorResponse::ServerError(_))));
    }

    #[test]
    fn parses_the_bearer_token_case_insensitively() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("bEaReR abc"),
        );

        assert_eq!(bearer_token(&headers), Some("abc"));
    }

    #[test]
    fn ignores_other_schemes_and_empty_tokens() {
        for value in ["Basic abc", "Bearer", "Bearer ", "abc"] {
            let mut headers = HeaderMap::new();
            headers.insert(header::AUTHORIZATION, HeaderValue::from_static(value));

            assert_eq!(bearer_token(&headers), None, "{value}");
        }
    }
}

#[cfg(test)]
mod cache_control_tests {
    use axum::{
        body::Body,
        http::{Request, header},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn sends_no_store_and_answers_get_and_post() {
        // A request without a token is rejected before touching any persistence.
        for request in [
            Request::get("/userinfo").body(Body::empty()).unwrap(),
            Request::post("/userinfo").body(Body::empty()).unwrap(),
        ] {
            let response = super::router().oneshot(request).await.unwrap();

            assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
            assert_eq!(
                response.headers().get(header::CACHE_CONTROL).unwrap(),
                "no-store"
            );
        }
    }
}
