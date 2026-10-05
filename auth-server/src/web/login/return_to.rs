/// Only paths on this server are allowed as `return_to`, so the login can't be used as an open
/// redirect. `//host` and `/\host` are rejected because browsers treat them as protocol-relative.
pub(super) fn is_local_path(return_to: &str) -> bool {
    return_to.starts_with('/')
        && !return_to.starts_with("//")
        && !return_to.contains(['\\', '\r', '\n'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_paths_on_this_server() {
        assert!(is_local_path(
            "/authorize?client_id=1&request_uri=urn%3Aabc"
        ));
    }

    #[test]
    fn rejects_return_to_values_leaving_this_server() {
        for return_to in [
            "",
            "https://evil.example",
            "//evil.example",
            "/\\evil.example",
            "authorize",
            "/ok\r\nSet-Cookie: a=b",
        ] {
            assert!(!is_local_path(return_to), "{return_to:?}");
        }
    }
}
