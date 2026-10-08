//! Parsing of the `#[ApiErrorResponse(..)]` arguments: `headers(..)` and `render = path`,
//! both optional, in any order, separated by commas.

use syn::parse::{Parse, ParseStream};
use syn::{Ident, Path, Token};

use crate::headers::Headers;

#[derive(Default)]
pub struct ErrorResponseArgs {
    pub headers: Headers,
    /// A function `fn(StatusCode, &str, &str) -> Response` rendering the status, error
    /// code and description. Without it the body is the JSON error object.
    pub render: Option<Path>,
}

impl Parse for ErrorResponseArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = Self::default();
        let mut seen_headers = false;

        while !input.is_empty() {
            let keyword: Ident = input.fork().parse()?;
            if keyword == "headers" {
                if seen_headers {
                    return Err(syn::Error::new(keyword.span(), "duplicate `headers(..)`"));
                }
                seen_headers = true;
                args.headers = input.parse()?;
            } else if keyword == "render" {
                if args.render.is_some() {
                    return Err(syn::Error::new(keyword.span(), "duplicate `render = ..`"));
                }
                input.parse::<Ident>()?;
                input.parse::<Token![=]>()?;
                args.render = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    keyword.span(),
                    "expected `headers(..)` or `render = path`",
                ));
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(args)
    }
}
