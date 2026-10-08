//! Building what a single enum variant expands into: its `match` arm, and for
//! `#[server_error]` variants a `From` implementation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, Expr, Fields, Ident, Type, Variant};

use super::attributes::{forbid, format_string, has, marker, optional, required};

/// What one variant contributes to the expansion.
pub struct VariantExpansion {
    /// The `match` arm producing the variant's status, error code and description, or
    /// returning its response directly.
    pub arm: TokenStream,
    /// The type of the source of a `#[server_error]` variant, to generate `From` for.
    pub from_source: Option<Type>,
}

/// The fixed error code and description of a `#[server_error]` variant. The source is
/// only logged, never shown to the client.
const SERVER_ERROR_CODE: &str = "server_error";
const SERVER_ERROR_DESCRIPTION: &str = "An unexpected error occurred while processing the request.";

pub fn expand(variant: &Variant) -> Result<VariantExpansion, Error> {
    if has(variant, "server_error") {
        server_error(variant)
    } else if has(variant, "into_response") {
        into_response(variant)
    } else {
        described(variant)
    }
}

/// `#[server_error] ServerError(Source)`: a 500 with a generic description. The source
/// is logged, so a failure is logged once, where it becomes a response.
fn server_error(variant: &Variant) -> Result<VariantExpansion, Error> {
    let mut errors: Option<Error> = None;
    optional(variant, "server_error", &mut errors, marker);
    forbid(
        variant,
        "server_error",
        &[
            "status_code",
            "error",
            "code",
            "description",
            "into_response",
        ],
        &mut errors,
    );
    let source = single_field(variant, "server_error", &mut errors);

    if let Some(err) = errors {
        return Err(err);
    }

    let ident = &variant.ident;
    Ok(VariantExpansion {
        arm: quote! {
            Self::#ident(__source) => {
                ::tracing::error!(error = ?__source, "request failed with a server error");
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    ::std::string::String::from(#SERVER_ERROR_CODE),
                    ::std::string::String::from(#SERVER_ERROR_DESCRIPTION),
                )
            }
        },
        from_source: source,
    })
}

/// `#[into_response] Variant(Inner)`: `Inner` is the response, e.g. a redirect.
fn into_response(variant: &Variant) -> Result<VariantExpansion, Error> {
    let mut errors: Option<Error> = None;
    optional(variant, "into_response", &mut errors, marker);
    forbid(
        variant,
        "into_response",
        &[
            "status_code",
            "error",
            "code",
            "description",
            "server_error",
        ],
        &mut errors,
    );
    single_field(variant, "into_response", &mut errors);

    if let Some(err) = errors {
        return Err(err);
    }

    let ident = &variant.ident;
    Ok(VariantExpansion {
        arm: quote! {
            Self::#ident(__inner) => return axum::response::IntoResponse::into_response(__inner),
        },
        from_source: None,
    })
}

/// A variant described by its attributes: either `#[code(..)]`, or `#[status_code(..)]`
/// together with `#[error(..)]`, and always `#[description(..)]`.
fn described(variant: &Variant) -> Result<VariantExpansion, Error> {
    let mut errors: Option<Error> = None;

    let ident = &variant.ident;
    let (pattern, args) = pattern_and_args(ident, &variant.fields);

    let status_and_code = if has(variant, "code") {
        forbid(variant, "code", &["status_code", "error"], &mut errors);
        optional(variant, "code", &mut errors, |attr| {
            attr.parse_args::<Expr>()
        })
        .map(|code| {
            (
                quote!((#code).status()),
                quote!(::std::string::String::from((#code).as_str())),
            )
        })
    } else {
        let status_code = required(variant, "status_code", &mut errors, |attr| {
            attr.parse_args::<Expr>()
        });
        let error_code = required(variant, "error", &mut errors, format_string);
        status_code
            .zip(error_code)
            .map(|(status_code, error_code)| {
                (
                    quote!(#status_code),
                    quote!(::std::format!(#error_code #(, #args)*)),
                )
            })
    };
    let description = required(variant, "description", &mut errors, format_string);

    if let Some(err) = errors {
        return Err(err);
    }

    let (status_code, error_code) = status_and_code.unwrap();
    let description = description.unwrap();

    Ok(VariantExpansion {
        arm: quote! {
            #[allow(unused_variables)]
            #pattern => (
                #status_code,
                #error_code,
                ::std::format!(#description #(, #args)*),
            ),
        },
        from_source: None,
    })
}

/// The pattern binding a variant's fields, and the bindings to pass positionally to a
/// format string.
fn pattern_and_args(ident: &Ident, fields: &Fields) -> (TokenStream, Vec<Ident>) {
    match fields {
        Fields::Unit => (quote!(Self::#ident), Vec::new()),
        // Named fields are bound under their own names, so a format string can pick
        // them up through implicit capture: `{email}`.
        Fields::Named(fields) => {
            let names = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().expect("named field has an ident"));
            (quote!(Self::#ident { #(#names),* }), Vec::new())
        }
        // Tuple fields are passed positionally instead, so `{0}` works like it does
        // in `thiserror`.
        Fields::Unnamed(fields) => {
            let bindings: Vec<_> = (0..fields.unnamed.len())
                .map(|index| format_ident!("__f{}", index))
                .collect();
            (quote!(Self::#ident( #(#bindings),* )), bindings)
        }
    }
}

/// The type of the single field of a tuple variant, which `marker` requires.
fn single_field(variant: &Variant, marker: &str, errors: &mut Option<Error>) -> Option<Type> {
    match &variant.fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Some(fields.unnamed[0].ty.clone()),
        _ => {
            super::combine(
                errors,
                Error::new(
                    variant.ident.span(),
                    format!(
                        "`#[{marker}]` requires a tuple variant with exactly one field, \
                         e.g. `{}(Inner)`",
                        variant.ident
                    ),
                ),
            );
            None
        }
    }
}
