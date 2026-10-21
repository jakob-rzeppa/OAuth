//! OpenAPI document assembly, served alongside the API by [`super::router`].

use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

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

/// Registers the `bearer_auth` security scheme referenced by the protected endpoints.
///
/// The access token is issued by the auth-server and checked by
/// [`crate::security::middleware::auth_middleware`].
struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .description(Some("Access token issued by the auth-server."))
                    .build(),
            ),
        );
    }
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
    components(schemas(ErrorBody)),
    modifiers(&BearerAuth)
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documents_bearer_auth_on_protected_endpoints_only() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();

        assert_eq!(
            spec["components"]["securitySchemes"]["bearer_auth"],
            serde_json::json!({
                "type": "http",
                "scheme": "bearer",
                "description": "Access token issued by the auth-server."
            })
        );
        assert_eq!(
            spec["paths"]["/v1/users"]["post"]["security"],
            serde_json::json!([{ "bearer_auth": ["identity:users:create"] }])
        );
        assert!(spec["paths"]["/health"]["get"].get("security").is_none());
    }
}
