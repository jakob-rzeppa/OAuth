use axum::{
    body::Body,
    http::{HeaderMap, Request, StatusCode},
};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{admin::users::router, domain::entity::user::User};

pub fn user(id: Uuid, user_name: &str) -> User {
    User::new(
        id,
        user_name.to_string(),
        "Alice".to_string(),
        "secret-hash".to_string(),
        false,
        vec![],
    )
    .unwrap()
}

/// Sends `request` to the `/admin/users` router.
pub async fn send(request: Request<Body>) -> (StatusCode, HeaderMap, String) {
    let response = router().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, headers, String::from_utf8(body.to_vec()).unwrap())
}

pub fn form(method: &str, uri: &str, body: &'static str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap()
}

pub fn empty(method: &str, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}
