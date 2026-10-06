use axum::http::HeaderValue;

/// Name of the cookie carrying the login session token.
pub(super) const LOGIN_SESSION_COOKIE: &str = "login_session";

/// Name of the cookie carrying the user session token, set once the user is logged in.
pub(super) const USER_SESSION_COOKIE: &str = "user_session";

/// The `Set-Cookie` value handing a session token to the browser for `max_age_seconds`.
/// The token is URL-safe base64, so it needs no cookie escaping.
pub(super) fn session_cookie(name: &str, token: &str, max_age_seconds: u64) -> Option<HeaderValue> {
    HeaderValue::from_str(&format!(
        "{name}={token}; Max-Age={max_age_seconds}; Path=/; HttpOnly; Secure; SameSite=Lax"
    ))
    .ok()
}

/// The `Set-Cookie` value telling the browser to delete the cookie `name`.
/// The attributes match `session_cookie`, so the browser treats it as the same cookie.
pub(super) fn expired_cookie(name: &str) -> Option<HeaderValue> {
    session_cookie(name, "", 0)
}

/// Finds the cookie `name` in the value of a `Cookie` header (`a=1; b=2`).
/// An empty value counts as missing.
pub(super) fn cookie_value<'a>(cookies: &'a str, name: &str) -> Option<&'a str> {
    cookies
        .split(';')
        .filter_map(|cookie| cookie.trim().split_once('='))
        .find(|(cookie_name, _)| *cookie_name == name)
        .map(|(_, value)| value)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_cookie_has_an_empty_value_and_no_lifetime() {
        let cookie = expired_cookie("login_session").unwrap();

        assert_eq!(
            cookie,
            "login_session=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax"
        );
    }

    #[test]
    fn finds_the_cookie_among_others() {
        assert_eq!(
            cookie_value("theme=dark; login_session=abc; lang=en", "login_session"),
            Some("abc")
        );
    }

    #[test]
    fn does_not_match_cookies_with_a_longer_name() {
        assert_eq!(cookie_value("x_login_session=abc", "login_session"), None);
    }

    #[test]
    fn treats_an_empty_value_as_missing() {
        assert_eq!(cookie_value("login_session=", "login_session"), None);
    }
}
