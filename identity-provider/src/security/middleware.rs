use std::pin::Pin;

use axum::{
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::security::{
    context::{AuthContext, client::AuthenticatedClient, user::AuthenticatedUser},
    error_response::AuthErrorResponse,
    introspect::{Introspection, introspect_token},
};

/// Middleware that authenticates a request with the auth-server.
///
/// Pass the required scopes and if a user is required for this route.
///
/// For multiple scopes add them with a space between.
///
/// Expects a Bearer token in the Authorization header and will perform token introspection against the auth-server.
/// Responds with:
/// - 401 Unauthorized if the request is not authenticated,
/// - 403 Forbidden if the token is valid, but not allowed to access this route,
/// - 503 Service Unavailable if the auth-server failed, so the token could not be checked.
///
/// Passes the authenticated user or client to the request extensions as an `AuthContext`.
pub fn auth_middleware(
    required_scope: &'static str,
    require_user: bool,
) -> impl Clone + Send + Fn(Request, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> {
    move |mut req: Request, next: Next| {
        Box::pin(async move {
            match authenticate(req.headers(), required_scope, require_user).await {
                Ok(auth_context) => {
                    req.extensions_mut().insert(auth_context);
                    next.run(req).await
                }
                Err(rejection) => rejection.into_response(),
            }
        })
    }
}

async fn authenticate(
    headers: &HeaderMap,
    required_scope: &str,
    require_user: bool,
) -> Result<AuthContext, AuthErrorResponse> {
    // Extract the Bearer Token
    let header_str = headers
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .ok_or(AuthErrorResponse::Unauthorized)?;
    let token = header_str
        .strip_prefix("Bearer ")
        .ok_or(AuthErrorResponse::Unauthorized)?;

    // A failing auth-server is not the client's fault: `?` turns it into a 503.
    let Introspection::Active {
        scope,
        client_id,
        sub,
    } = introspect_token(token).await?
    else {
        return Err(AuthErrorResponse::Unauthorized);
    };

    let auth_context = if let Some(user_id) = sub {
        // We have a user
        AuthContext::User(AuthenticatedUser::new(client_id, user_id, scope))
    } else {
        AuthContext::Client(AuthenticatedClient::new(client_id, scope))
    };

    // The token is valid, but not allowed to access this route.
    if (require_user && !auth_context.is_user())
        || !auth_context.has_scopes(&required_scope.split_whitespace().collect::<Vec<_>>())
    {
        return Err(AuthErrorResponse::Forbidden);
    }

    Ok(auth_context)
}
