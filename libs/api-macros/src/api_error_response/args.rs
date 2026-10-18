//! Parsing of the `#[ApiErrorResponse(..)]` arguments: an optional `headers(..)`.

use syn::parse::{Parse, ParseStream};
use syn::{Ident, Token};

use crate::headers::Headers;

#[derive(Default)]
pub struct ErrorResponseArgs {
    pub headers: Headers,
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
            } else {
                return Err(syn::Error::new(keyword.span(), "expected `headers(..)`"));
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(args)
    }
}
