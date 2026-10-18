//! OpenAPI document assembly, served alongside the API by [`super::router`].

use utoipa::OpenApi;

use crate::api::{health, roles, users};

/// Documents the `{"error": ..., "error_description": ...}` body produced by every
/// `#[ApiErrorResponse]` enum. The enums themselves can't derive `ToSchema` - their
/// variants are turned into that shape at runtime by the macro, not by serialising
/// the enum directly - so this stands in for them in the generated spec.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub struct ErrorBody {
    /// Machine-readable error code, e.g. `"user_not_found"`.
    pub error: String,
    /// Human-readable description of the error.
    pub error_description: String,
}

#[derive(OpenApi)]
#[openapi(
    info(title = "Identity Provider", version = env!("CARGO_PKG_VERSION")),
    paths(
        health::health_endpoint,
        users::create::create_user_endpoint,
        users::get::get_user_endpoint,
        users::query::query_users_endpoint,
        users::update::update_user_endpoint,
        users::delete::delete_user_endpoint,
        users::authenticate::authenticate_user_endpoint,
        users::password::set::set_password_endpoint,
        users::password::reset::reset_password_endpoint,
        roles::query::query_roles_endpoint
    ),
    components(schemas(ErrorBody))
)]
pub struct ApiDoc;
