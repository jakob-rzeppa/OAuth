# auth

Axum middleware that protects a resource server's routes with the auth-server. It reads the Bearer access token from a request, validates it through token introspection ([RFC 7662](https://datatracker.ietf.org/doc/html/rfc7662)) and hands the result to your handlers as an `AuthContext`.

## Setup

Set the base URL of the auth-server:

```
AUTH_SERVER_URL=https://auth.example.com
```

Call `auth::init()` once at startup. It reads the configuration right away, so a missing `AUTH_SERVER_URL` panics at startup and not during the first request.

```rust
auth::init();
```

## Protecting routes

`auth::middleware::auth_middleware(required_scope, require_user)` returns a middleware for `axum::middleware::from_fn`.

- `required_scope`: the scopes the token must have. Separate several scopes with a space, e.g. `"read write"`. The token must have all of them.
- `require_user`: if `true`, the token must have been issued on behalf of a user. Tokens that a client got for itself are rejected.

```rust
use axum::{Extension, Router, middleware::from_fn, routing::get};
use auth::{context::AuthContext, middleware::auth_middleware};

async fn whoami(Extension(ctx): Extension<AuthContext>) -> String {
    format!("client {}, user {:?}", ctx.client_id(), ctx.user_id())
}

let app: Router = Router::new()
    .route("/me", get(whoami))
    .layer(from_fn(auth_middleware("profile:read", true)));
```

## AuthContext

After a successful check the middleware inserts an `AuthContext` into the request extensions. It is one of:

- `AuthContext::User`: a client acting on behalf of a user (the token has a `sub`).
- `AuthContext::Client`: a client acting by itself.

| Method                      | Description                                   |
| --------------------------- | --------------------------------------------- |
| `is_user()` / `is_client()` | Which kind of context this is                 |
| `client_id()`               | The id of the client the token was issued to  |
| `user_id()`                 | The user's id, or `None` for a client context |
| `has_scope(scope)`          | Whether the token has this scope              |
| `has_scopes(&[scopes])`     | Whether the token has all of these scopes     |

Use `has_scope` in a handler for checks that depend on the request and so can't be decided per route.

## Responses

A request the middleware rejects gets a JSON body of the form `{"error": "...", "error_description": "..."}`.

| Status | `error`               | When                                                                                                                                                               |
| ------ | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 401    | `unauthorized`        | The `Authorization: Bearer <token>` header is missing or malformed, or the auth-server reports the token as inactive. The response has `WWW-Authenticate: Bearer`. |
| 403    | `forbidden`           | The token is valid but lacks a required scope, or a user is required and the token has none.                                                                       |
| 503    | `service_unavailable` | The auth-server could not be reached or answered with an error. This says nothing about the token, so clients should retry and not discard it.                     |

## How it works

The middleware sends every request's token to `POST {AUTH_SERVER_URL}/introspect` as a form field `token`. A token counts as active only if the response has `active: true` together with a `scope` and a `client_id`. Nothing is cached, so a revoked token stops working right away, at the cost of one auth-server call per request.
