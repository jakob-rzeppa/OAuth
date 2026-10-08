//! Expansion of the `#[ApiErrorResponse]` attribute macro.

mod args;
mod attributes;
mod variant;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, ItemEnum};

pub use args::ErrorResponseArgs;

/// Re-emits the enum without its helper attributes and appends the generated
/// `IntoResponse` implementation, plus a `From` implementation for the source of every
/// `#[server_error]` variant.
pub fn expand(args: ErrorResponseArgs, mut item: ItemEnum) -> Result<TokenStream, Error> {
    let mut errors: Option<Error> = None;
    let mut arms = Vec::with_capacity(item.variants.len());
    let mut from_impls = Vec::new();

    // Cloned, because the variants are rewritten below while these are still in use.
    let ident = item.ident.clone();
    let generics = item.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    for variant in &item.variants {
        match variant::expand(variant) {
            Ok(expansion) => {
                arms.push(expansion.arm);
                if let Some(source) = expansion.from_source {
                    let variant_ident = &variant.ident;
                    from_impls.push(quote! {
                        impl #impl_generics ::std::convert::From<#source> for #ident #ty_generics #where_clause {
                            fn from(source: #source) -> Self {
                                Self::#variant_ident(source)
                            }
                        }
                    });
                }
            }
            Err(err) => combine(&mut errors, err),
        }
    }

    if let Some(err) = errors {
        return Err(err);
    }

    // An attribute macro replaces the item it is applied to, so the enum has to be
    // emitted again - without the helper attributes, which no longer exist by then.
    for variant in &mut item.variants {
        variant.attrs.retain(|attr| !attributes::is_helper(attr));
    }

    let headers = args.headers.to_tokens();
    let response = match (&args.render, headers) {
        (None, Some(headers)) => quote! {
            let body = axum::Json(::serde_json::json!({
                "error": error_code,
                "error_description": error_description,
            }));
            axum::response::IntoResponse::into_response((status, #headers, body))
        },
        (None, None) => quote! {
            let body = axum::Json(::serde_json::json!({
                "error": error_code,
                "error_description": error_description,
            }));
            axum::response::IntoResponse::into_response((status, body))
        },
        (Some(render), Some(headers)) => quote! {
            let response: axum::response::Response = #render(status, &error_code, &error_description);
            axum::response::IntoResponse::into_response((#headers, response))
        },
        (Some(render), None) => quote! {
            #render(status, &error_code, &error_description)
        },
    };

    Ok(quote! {
        #item

        #(#from_impls)*

        impl #impl_generics axum::response::IntoResponse for #ident #ty_generics #where_clause {
            fn into_response(self) -> axum::response::Response {
                let (status, error_code, error_description): (
                    axum::http::StatusCode,
                    ::std::string::String,
                    ::std::string::String,
                ) = match self {
                    #(#arms)*
                };

                #response
            }
        }
    })
}

/// Accumulates `err` so that every problem in the enum is reported at once, rather
/// than only the first one.
fn combine(errors: &mut Option<Error>, err: Error) {
    match errors {
        Some(existing) => existing.combine(err),
        None => *errors = Some(err),
    }
}
