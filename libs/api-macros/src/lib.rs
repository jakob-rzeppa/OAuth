#![allow(non_snake_case)]

//! Procedural macros for building HTTP API types.

mod api_error_response;
mod api_request;
mod api_response;
mod headers;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{Error, Expr, ItemEnum, ItemStruct};

use api_error_response::ErrorResponseArgs;
use api_response::ApiResponseArgs;

/// Generates an [`axum::response::IntoResponse`] implementation for an error enum.
///
/// Every variant is described by one of:
///
/// - `#[status_code(..)]`, `#[error(..)]` and `#[description(..)]`. The error code and
///   description are format strings in the style of `thiserror`, so they may interpolate
///   the variant's own fields: named fields by name (`{email}`), tuple fields by
///   position (`{0}`).
/// - `#[code(..)]` and `#[description(..)]`. `code` is an expression of a type with
///   `fn status(&self) -> StatusCode` and `fn as_str(&self) -> &str` methods, typically
///   an enum of the error codes a protocol defines, so the status always matches the code.
/// - `#[server_error]` on a tuple variant with one field, the source of the failure.
///   It responds with a 500, the code `server_error` and a generic description; the
///   source is logged with `tracing::error!` (so the crate must depend on `tracing`)
///   and never shown to the client. A `From<Source>` implementation is generated, so
///   `?` converts the source into the variant.
/// - `#[into_response]` on a tuple variant with one field implementing
///   [`axum::response::IntoResponse`], which becomes the whole response (e.g. a redirect).
///
/// ```ignore
/// #[ApiErrorResponse]
/// pub enum CreateUserErrorResponse {
///     #[status_code(axum::http::StatusCode::BAD_REQUEST)]
///     #[error("invalid_request")]
///     #[description("Invalid request body.")]
///     InvalidBody,
///
///     #[code(OAuthErrorCode::InvalidClient)]
///     #[description("The client was not found.")]
///     ClientNotFound,
///
///     #[server_error]
///     ServerError(InternalError),
/// }
/// ```
///
/// The response body is `{"error": <code>, "error_description": <description>}`.
///
/// The macro optionally takes, in any order:
///
/// - `headers(<name> => <value>, ..)`, which adds the given headers to the response of
///   every variant. Each name is a [`axum::http::HeaderName`] expression and each value
///   anything convertible into an [`axum::http::HeaderValue`].
/// - `render = <path>`, a function `fn(StatusCode, &str, &str) -> Response` that renders
///   the status, error code and description instead of the JSON body, e.g. as an HTML page.
///
/// ```ignore
/// #[ApiErrorResponse(headers(axum::http::header::CACHE_CONTROL => "no-store"))]
/// pub enum CreateUserErrorResponse { /* .. */ }
///
/// #[ApiErrorResponse(render = crate::util::html::render_error_page)]
/// pub enum LoginErrorResponse { /* .. */ }
/// ```
#[proc_macro_attribute]
pub fn ApiErrorResponse(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = if attr.is_empty() {
        ErrorResponseArgs::default()
    } else {
        match syn::parse2::<ErrorResponseArgs>(TokenStream2::from(attr)) {
            Ok(args) => args,
            Err(err) => return err.to_compile_error().into(),
        }
    };

    let item = syn::parse_macro_input!(item as ItemEnum);

    match api_error_response::expand(args, item) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generates an [`axum::response::IntoResponse`] implementation for a success response
/// struct and derives [`serde::Serialize`] on it.
///
/// The first argument is the [`axum::http::StatusCode`] expression the response should
/// carry. The struct is serialised into the JSON body. It may be followed by
/// `headers(<name> => <value>, ..)` to add response headers; each name is a
/// [`axum::http::HeaderName`] expression and each value anything convertible into an
/// [`axum::http::HeaderValue`].
///
/// ```ignore
/// #[ApiResponse(axum::http::StatusCode::CREATED)]
/// pub struct CreateUserResponse {
///     pub id: String,
/// }
///
/// #[ApiResponse(
///     axum::http::StatusCode::OK,
///     headers(axum::http::header::CACHE_CONTROL => "no-store")
/// )]
/// pub struct TokenResponse {
///     pub access_token: String,
/// }
/// ```
#[proc_macro_attribute]
pub fn ApiResponse(attr: TokenStream, item: TokenStream) -> TokenStream {
    if attr.is_empty() {
        return Error::new(
            proc_macro2::Span::call_site(),
            "`ApiResponse` takes a status code expression, e.g. `#[ApiResponse(axum::http::StatusCode::CREATED)]`",
        )
        .to_compile_error()
        .into();
    }

    let args = syn::parse_macro_input!(attr as ApiResponseArgs);
    let item = syn::parse_macro_input!(item as ItemStruct);

    match api_response::expand(args, item) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generates an [`axum::extract::FromRequest`] implementation for a request struct
/// and derives [`serde::Deserialize`] on it.
///
/// The implementation reads the entire request body and deserialises it from JSON.
/// Any failure - reading the body or deserialising it - is turned into the error
/// given as the macro's single argument, which must be an enum-variant path. The
/// enum it names becomes the rejection type.
///
/// ```ignore
/// #[ApiRequest(UpdateUserErrorResponse::InvalidBody)]
/// pub struct UpdateUserRequest {
///     pub email: Option<String>,
/// }
/// ```
#[proc_macro_attribute]
pub fn ApiRequest(attr: TokenStream, item: TokenStream) -> TokenStream {
    if attr.is_empty() {
        return Error::new(
            proc_macro2::Span::call_site(),
            "`ApiRequest` takes an enum-variant path, e.g. `#[ApiRequest(UpdateUserErrorResponse::InvalidBody)]`",
        )
        .to_compile_error()
        .into();
    }

    let error = syn::parse_macro_input!(attr as Expr);
    let item = syn::parse_macro_input!(item as ItemStruct);

    match api_request::expand(error, item) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
