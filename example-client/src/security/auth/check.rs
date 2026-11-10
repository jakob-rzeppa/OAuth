//! A value the auth-server sends back that the client compares against what it expects.

/// One comparison, kept so the flow page can show what was expected and what came back.
#[derive(Debug, Clone)]
pub struct Check {
    /// The parameter or claim, e.g. `state` or `iss`.
    pub name: &'static str,
    pub expected: String,
    /// `None` when the value was missing.
    pub actual: Option<String>,
}

impl Check {
    pub fn new(name: &'static str, expected: impl Into<String>, actual: Option<String>) -> Self {
        Self { name, expected: expected.into(), actual }
    }

    pub fn passed(&self) -> bool {
        self.actual.as_deref() == Some(self.expected.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_when_the_value_matches() {
        assert!(Check::new("iss", "https://a", Some("https://a".to_string())).passed());
    }

    #[test]
    fn fails_when_the_value_differs_or_is_missing() {
        assert!(!Check::new("iss", "https://a", Some("https://b".to_string())).passed());
        assert!(!Check::new("iss", "https://a", None).passed());
    }
}
