/// The scope that turns an authorization request into an OpenID Connect request.
pub const OPENID_SCOPE: &str = "openid";

/// The scope that grants access to the user's profile claims (`name`, `preferred_username`).
pub const PROFILE_SCOPE: &str = "profile";

pub fn has_openid_scope(scope: &str) -> bool {
    has_scope(scope, OPENID_SCOPE)
}

pub fn has_profile_scope(scope: &str) -> bool {
    has_scope(scope, PROFILE_SCOPE)
}

fn has_scope(scope: &str, wanted: &str) -> bool {
    scope.split_whitespace().any(|scope| scope == wanted)
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

    #[test]
    fn detects_the_profile_scope_among_others() {
        assert!(has_profile_scope("openid profile"));
        assert!(!has_profile_scope("openid"));
        assert!(!has_profile_scope("profiles"));
    }
}
