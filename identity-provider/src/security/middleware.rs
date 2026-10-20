use std::pin::Pin;

use axum::{extract::Request, middleware::Next, response::Response};

use crate::security::{
    context::{AuthContext, client::AuthenticatedClient, user::AuthenticatedUser},
    introspect::{Introspection, introspect_token},
};

/// Middleware that authenticates a request with the auth-server.
///
/// Pass the required scopes and if a user is required for this route.
///
/// For multiple scopes add them with a space between.
///
/// Expects a Bearer token in the Authorization header and will perform token introspection against the auth-server.
/// Provides a 401 Unauthorized response if the request is not authenticated.
///
/// Passes the authenticated user or client to the request extensions as an `AuthContext`.
pub fn auth_middleware(
    required_scope: &'static str,
    require_user: bool,
) -> impl Clone + Send + Fn(Request, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> {
    move |mut req: Request, next: Next| {
        Box::pin(async move {
            // Reject the request with a 401 Unauthorized response
            // The WWW-Authenticate header is set to "Bearer" to indicate that the client should provide a Bearer token for authentication.
            // No other information is provided in the response body.
            fn reject_request() -> Response {
                Response::builder()
                    .status(401)
                    .header("WWW-Authenticate", "Bearer")
                    .body(axum::body::Body::empty())
                    .unwrap()
            }

            // Extract the Bearer Token
            let Some(auth_header) = req.headers().get("Authorization") else {
                return reject_request();
            };
            let header_str = auth_header.to_str().unwrap_or("");
            let token = if header_str.starts_with("Bearer ") {
                header_str.trim_start_matches("Bearer ").to_string()
            } else {
                return reject_request();
            };

            let introspection = match introspect_token(&token).await {
                Ok(introspection) => introspection,
                Err(e) => {
                    tracing::error!("Token introspection failed unexpectedly: {}", e);
                    return reject_request();
                }
            };
            let Introspection::Active {
                scope,
                client_id,
                sub,
            } = introspection
            else {
                return reject_request();
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
                return Response::builder()
                    .status(403)
                    .body(axum::body::Body::empty())
                    .unwrap();
            }

            req.extensions_mut().insert(auth_context);

            next.run(req).await
        })
    }
}
