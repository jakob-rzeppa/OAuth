pub mod save;

fn key(session_token_hash: &str) -> String {
    format!("session:user:{session_token_hash}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_namespaced_by_session_token_hash() {
        assert_eq!(key("abc"), "session:user:abc");
    }
}
