/// The scope that turns an authorization request into an OpenID Connect request.
pub const OPENID_SCOPE: &str = "openid";

pub fn has_openid_scope(scope: &str) -> bool {
    scope.split_whitespace().any(|scope| scope == OPENID_SCOPE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_the_openid_scope_among_others() {
        assert!(has_openid_scope("openid"));
        assert!(has_openid_scope("read openid profile"));
        assert!(!has_openid_scope("read write"));
        assert!(!has_openid_scope("openid2"));
        assert!(!has_openid_scope(""));
    }
}
