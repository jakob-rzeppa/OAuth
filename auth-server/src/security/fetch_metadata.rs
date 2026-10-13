use axum::{
    extract::Request,
    http::{HeaderMap, Method, StatusCode, header::ORIGIN},
    middleware::Next,
    response::Response,
};

use crate::{config::Config, util::html::render_error_page};

const SEC_FETCH_SITE: &str = "sec-fetch-site";

/// Request layer for the browser forms: rejects state-changing requests that did not come from
/// the auth-server's own pages, as a second CSRF layer next to the form tokens.
///
/// Browsers set `Sec-Fetch-Site` and `Origin` themselves and scripts cannot override them.
/// Requests carrying neither header (non-browser clients) are let through, since the cookies and
/// tokens still apply to them and we don't have to worry about CSRF attacks from them.
pub async fn reject_cross_site(request: Request, next: Next) -> Response {
    let decision = decide(request.method(), request.headers(), Config::issuer_origin);

    if let Err(rejected) = decision {
        // Only the method and path: the query string of `/authorize` carries the `request_uri`.
        tracing::warn!(
            method = %request.method(),
            path = request.uri().path(),
            header = rejected.header(),
            "rejected a cross-site request"
        );
        return render_error_page(
            StatusCode::FORBIDDEN,
            "invalid_request",
            "This request did not come from this site.",
        );
    }

    next.run(request).await
}

/// The header that made [`decide`] reject a request.
#[derive(Debug, PartialEq, Eq)]
enum Rejected {
    SecFetchSite,
    Origin,
}

impl Rejected {
    fn header(&self) -> &'static str {
        match self {
            Rejected::SecFetchSite => "Sec-Fetch-Site",
            Rejected::Origin => "Origin",
        }
    }
}

/// `Sec-Fetch-Site` wins when present, and only `same-origin` (and `none`, for user-initiated
/// requests) pass: `same-site` would let a sibling subdomain through. `Origin` is the fallback
/// for browsers that do not send it. The issuer origin is only looked up when it is needed.
fn decide(
    method: &Method,
    headers: &HeaderMap,
    issuer_origin: impl FnOnce() -> &'static str,
) -> Result<(), Rejected> {
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return Ok(());
    }

    if let Some(site) = headers.get(SEC_FETCH_SITE) {
        return match site.as_bytes() {
            b"same-origin" | b"none" => Ok(()),
            _ => Err(Rejected::SecFetchSite),
        };
    }

    if let Some(origin) = headers.get(ORIGIN) {
        return if origin.as_bytes() == issuer_origin().as_bytes() {
            Ok(())
        } else {
            Err(Rejected::Origin)
        };
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    const ISSUER_ORIGIN: &str = "https://issuer.example";

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(*name, HeaderValue::from_static(value));
        }
        headers
    }

    fn post(pairs: &[(&'static str, &'static str)]) -> Result<(), Rejected> {
        decide(&Method::POST, &headers(pairs), || ISSUER_ORIGIN)
    }

    #[test]
    fn allows_same_origin_and_user_initiated_requests() {
        assert_eq!(post(&[("sec-fetch-site", "same-origin")]), Ok(()));
        assert_eq!(post(&[("sec-fetch-site", "none")]), Ok(()));
    }

    #[test]
    fn rejects_same_site_and_cross_site_requests() {
        assert_eq!(
            post(&[("sec-fetch-site", "same-site")]),
            Err(Rejected::SecFetchSite)
        );
        assert_eq!(
            post(&[("sec-fetch-site", "cross-site")]),
            Err(Rejected::SecFetchSite)
        );
    }

    #[test]
    fn rejects_unknown_sec_fetch_site_values() {
        assert_eq!(
            post(&[("sec-fetch-site", "bogus")]),
            Err(Rejected::SecFetchSite)
        );
    }

    #[test]
    fn falls_back_to_the_origin_header_without_sec_fetch_site() {
        assert_eq!(post(&[("origin", ISSUER_ORIGIN)]), Ok(()));
        assert_eq!(
            post(&[("origin", "https://evil.example")]),
            Err(Rejected::Origin)
        );
        assert_eq!(post(&[("origin", "null")]), Err(Rejected::Origin));
    }

    #[test]
    fn rejects_an_origin_that_only_starts_with_the_issuer_origin() {
        assert_eq!(
            post(&[("origin", "https://issuer.example.evil.example")]),
            Err(Rejected::Origin)
        );
    }

    #[test]
    fn allows_requests_without_either_header() {
        assert_eq!(post(&[]), Ok(()));
    }

    #[test]
    fn sec_fetch_site_wins_over_origin() {
        assert_eq!(
            post(&[
                ("sec-fetch-site", "same-origin"),
                ("origin", "https://evil.example"),
            ]),
            Ok(())
        );
        assert_eq!(
            post(&[("sec-fetch-site", "cross-site"), ("origin", ISSUER_ORIGIN),]),
            Err(Rejected::SecFetchSite)
        );
    }

    #[test]
    fn lets_safe_methods_through() {
        let cross_site = headers(&[("sec-fetch-site", "cross-site")]);

        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert_eq!(decide(&method, &cross_site, || ISSUER_ORIGIN), Ok(()));
        }
    }

    #[test]
    fn checks_every_other_method() {
        let cross_site = headers(&[("sec-fetch-site", "cross-site")]);

        for method in [Method::POST, Method::PUT, Method::PATCH, Method::DELETE] {
            assert_eq!(
                decide(&method, &cross_site, || ISSUER_ORIGIN),
                Err(Rejected::SecFetchSite)
            );
        }
    }
}
