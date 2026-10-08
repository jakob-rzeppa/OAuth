//! Reading the helper attributes off a variant: `#[status_code(..)]`, `#[error(..)]`,
//! `#[code(..)]`, `#[description(..)]`, `#[server_error]` and `#[into_response]`.

use syn::{Attribute, Error, LitStr, Meta, Variant, spanned::Spanned};

use super::combine;

const HELPERS: [&str; 6] = [
    "status_code",
    "error",
    "code",
    "description",
    "server_error",
    "into_response",
];

/// All occurrences of `name` on `variant`.
pub fn find<'a>(variant: &'a Variant, name: &'a str) -> impl Iterator<Item = &'a Attribute> {
    variant
        .attrs
        .iter()
        .filter(move |attr| attr.path().is_ident(name))
}

/// Whether `variant` carries `name`.
pub fn has(variant: &Variant, name: &str) -> bool {
    find(variant, name).next().is_some()
}

/// Reads the single occurrence of `name` on `variant`, recording a problem in
/// `errors` if it is missing or repeated.
pub fn required<T>(
    variant: &Variant,
    name: &str,
    errors: &mut Option<Error>,
    parse: impl Fn(&Attribute) -> Result<T, Error>,
) -> Option<T> {
    let found = optional(variant, name, errors, parse);

    if !has(variant, name) {
        combine(
            errors,
            Error::new(
                variant.ident.span(),
                format!(
                    "missing `#[{name}(..)]` attribute on variant `{}`",
                    variant.ident
                ),
            ),
        );
    }

    found
}

/// Reads the occurrence of `name` on `variant` if there is one, recording a problem in
/// `errors` if it is repeated or can't be parsed.
pub fn optional<T>(
    variant: &Variant,
    name: &str,
    errors: &mut Option<Error>,
    parse: impl Fn(&Attribute) -> Result<T, Error>,
) -> Option<T> {
    let mut found = None;
    let mut seen = false;

    for attr in find(variant, name) {
        if seen {
            combine(
                errors,
                Error::new_spanned(attr, format!("duplicate `#[{name}(..)]` attribute")),
            );
            continue;
        }
        seen = true;

        match parse(attr) {
            Ok(value) => found = Some(value),
            // The attribute is there but unusable; the parse error already says so,
            // so don't also report it as missing.
            Err(err) => combine(errors, err),
        }
    }

    found
}

/// Reports every attribute in `names` present on `variant` as not allowed together
/// with `marker`.
pub fn forbid(variant: &Variant, marker: &str, names: &[&str], errors: &mut Option<Error>) {
    for name in names {
        for attr in find(variant, name) {
            combine(
                errors,
                Error::new_spanned(
                    attr,
                    format!("`#[{name}(..)]` can't be combined with `#[{marker}]`"),
                ),
            );
        }
    }
}

/// Checks that a marker attribute (`#[server_error]`, `#[into_response]`) has no arguments.
pub fn marker(attr: &Attribute) -> Result<(), Error> {
    match &attr.meta {
        Meta::Path(_) => Ok(()),
        _ => Err(Error::new(
            attr.span(),
            format!(
                "`#[{}]` takes no arguments",
                attr.path()
                    .get_ident()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            ),
        )),
    }
}

/// Parses an attribute whose body must be exactly one format string.
pub fn format_string(attr: &Attribute) -> Result<LitStr, Error> {
    attr.parse_args::<LitStr>().map_err(|_| {
        Error::new(
            attr.span(),
            format!(
                "`#[{}(..)]` takes exactly one string literal, which may interpolate \
                 the variant's fields (`{{name}}` for named fields, `{{0}}` for tuple fields)",
                attr.path()
                    .get_ident()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            ),
        )
    })
}

/// Whether `attr` is one of the helpers this macro consumes, and so must be stripped
/// from the re-emitted enum.
pub fn is_helper(attr: &Attribute) -> bool {
    HELPERS.iter().any(|name| attr.path().is_ident(name))
}
