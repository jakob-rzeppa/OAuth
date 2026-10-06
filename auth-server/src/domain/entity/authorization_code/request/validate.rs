use uuid::Uuid;

use crate::domain::entity::{authorization_code::request::AuthorizationRequest, client::Client};

pub struct ValidatedAuthorizationRequest {
    pub client_id: Uuid,
    pub redirect_uri: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

impl AuthorizationRequest {
    pub fn validate_and_unpack(
        self,
        client: &Client,
    ) -> Result<ValidatedAuthorizationRequest, ValidationError> {
        self.validate_against_client(client)?;

        let AuthorizationRequest {
            client_id,
            redirect_uri,
            response_type: _,
            scope,
            state,
            code_challenge,
            code_challenge_method,
        } = self;

        Ok(ValidatedAuthorizationRequest {
            client_id,
            redirect_uri,
            scope,
            state,
            code_challenge,
            code_challenge_method,
        })
    }

    pub fn validate_against_client(&self, client: &Client) -> Result<(), ValidationError> {
        // ==== client id ====
        if self.client_id() != client.id() {
            return Err(ValidationError::fatal(
                FatalValidationError::ClientIdMismatch,
            ));
        }

        // ==== redirect_uri ====
        if !client.has_redirect_uri(self.redirect_uri()) {
            tracing::warn!(
                %self.client_id,
                "authorization request rejected: redirect_uri is not registered for the client"
            );
            return Err(ValidationError::fatal(
                FatalValidationError::InvalidRedirectUri,
            ));
        }

        // ==== state ====
        if self.state().is_empty() {
            return Err(ValidationError::fatal(FatalValidationError::InvalidState));
        }

        // Only allow error redirects from here on, since the client_id and redirect_uri are valid.

        // ==== response type ====
        if self.response_type() != "code" {
            return Err(ValidationError::redirectable(
                RedirectableValidationError::InvalidResponseType,
                self.redirect_uri().to_string(),
                self.state().to_string(),
            ));
        }

        // ==== scope ====
        if !client.has_scope(self.scope()) {
            tracing::warn!(
                %self.client_id,
                "authorization request rejected: scope is not allowed for the client"
            );
            return Err(ValidationError::redirectable(
                RedirectableValidationError::InvalidScope,
                self.redirect_uri().to_string(),
                self.state().to_string(),
            ));
        }

        // ==== code_challenge_method ====
        if self.code_challenge_method() != "S256" {
            return Err(ValidationError::redirectable(
                RedirectableValidationError::InvalidCodeChallengeMethod,
                self.redirect_uri().to_string(),
                self.state().to_string(),
            ));
        }

        // === code_challenge ====
        if !is_valid_s256_code_challenge(self.code_challenge()) {
            return Err(ValidationError::redirectable(
                RedirectableValidationError::InvalidCodeChallenge,
                self.redirect_uri().to_string(),
                self.state().to_string(),
            ));
        }

        Ok(())
    }
}

/// An S256 code challenge is the base64url encoding (without padding) of a SHA-256 hash,
/// so it is always exactly 43 characters from the base64url alphabet (RFC 7636 §4.2).
fn is_valid_s256_code_challenge(code_challenge: &str) -> bool {
    code_challenge.len() == 43
        && code_challenge
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

#[derive(Debug, PartialEq)]
pub enum FatalValidationError {
    ClientIdMismatch,
    InvalidRedirectUri,
    InvalidState,
}

#[derive(Debug, PartialEq)]
pub enum RedirectableValidationError {
    InvalidResponseType,
    InvalidScope,
    InvalidCodeChallengeMethod,
    InvalidCodeChallenge,
}

#[derive(Debug, PartialEq)]
pub enum ValidationError {
    Fatal {
        error: FatalValidationError,
    },
    Redirectable {
        error: RedirectableValidationError,
        redirect_uri: String,
        state: String,
    },
}

impl ValidationError {
    pub fn fatal(error: FatalValidationError) -> Self {
        ValidationError::Fatal { error }
    }

    pub fn redirectable(
        error: RedirectableValidationError,
        redirect_uri: String,
        state: String,
    ) -> Self {
        ValidationError::Redirectable {
            error,
            redirect_uri,
            state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_client(id: Uuid) -> Client {
        Client::new(
            id,
            "Test Client".to_string(),
            vec!["https://example.com/callback".to_string()],
            vec!["read".to_string(), "write".to_string()],
        )
    }

    fn make_authorization_request(client_id: Uuid) -> AuthorizationRequest {
        AuthorizationRequest::new(
            client_id,
            "https://example.com/callback".to_string(),
            "code".to_string(),
            "read".to_string(),
            "some-state".to_string(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            "S256".to_string(),
        )
    }

    #[test]
    fn valid_request_passes_validation() {
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);
        let request = make_authorization_request(client_id);

        assert_eq!(request.validate_against_client(&client), Ok(()));
    }

    #[test]
    fn rejects_client_id_mismatch() {
        let client = make_client(Uuid::new_v4());
        let request = make_authorization_request(Uuid::new_v4());

        assert_eq!(
            request.validate_against_client(&client),
            Err(ValidationError::fatal(
                FatalValidationError::ClientIdMismatch
            ))
        );
    }

    #[test]
    fn rejects_invalid_response_type() {
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);
        let request = AuthorizationRequest::new(
            client_id,
            "https://example.com/callback".to_string(),
            "token".to_string(),
            "read".to_string(),
            "some-state".to_string(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            "S256".to_string(),
        );

        assert_eq!(
            request.validate_against_client(&client),
            Err(ValidationError::redirectable(
                RedirectableValidationError::InvalidResponseType,
                "https://example.com/callback".to_string(),
                "some-state".to_string(),
            ))
        );
    }

    #[test]
    fn rejects_unregistered_redirect_uri() {
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);
        let request = AuthorizationRequest::new(
            client_id,
            "https://evil.example.com/callback".to_string(),
            "code".to_string(),
            "read".to_string(),
            "some-state".to_string(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            "S256".to_string(),
        );

        assert_eq!(
            request.validate_against_client(&client),
            Err(ValidationError::fatal(
                FatalValidationError::InvalidRedirectUri,
            ))
        );
    }

    #[test]
    fn rejects_unregistered_scope() {
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);
        let request = AuthorizationRequest::new(
            client_id,
            "https://example.com/callback".to_string(),
            "code".to_string(),
            "delete".to_string(),
            "some-state".to_string(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            "S256".to_string(),
        );

        assert_eq!(
            request.validate_against_client(&client),
            Err(ValidationError::redirectable(
                RedirectableValidationError::InvalidScope,
                "https://example.com/callback".to_string(),
                "some-state".to_string(),
            ))
        );
    }

    #[test]
    fn rejects_empty_state() {
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);
        let request = AuthorizationRequest::new(
            client_id,
            "https://example.com/callback".to_string(),
            "code".to_string(),
            "read".to_string(),
            "".to_string(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            "S256".to_string(),
        );

        assert_eq!(
            request.validate_against_client(&client),
            Err(ValidationError::fatal(FatalValidationError::InvalidState))
        );
    }

    #[test]
    fn rejects_non_s256_code_challenge_method() {
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);
        let request = AuthorizationRequest::new(
            client_id,
            "https://example.com/callback".to_string(),
            "code".to_string(),
            "read".to_string(),
            "some-state".to_string(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            "plain".to_string(),
        );

        assert_eq!(
            request.validate_against_client(&client),
            Err(ValidationError::redirectable(
                RedirectableValidationError::InvalidCodeChallengeMethod,
                "https://example.com/callback".to_string(),
                "some-state".to_string(),
            ))
        );
    }

    #[test]
    fn rejects_empty_code_challenge() {
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);
        let request = AuthorizationRequest::new(
            client_id,
            "https://example.com/callback".to_string(),
            "code".to_string(),
            "read".to_string(),
            "some-state".to_string(),
            "".to_string(),
            "S256".to_string(),
        );

        assert_eq!(
            request.validate_against_client(&client),
            Err(ValidationError::redirectable(
                RedirectableValidationError::InvalidCodeChallenge,
                "https://example.com/callback".to_string(),
                "some-state".to_string(),
            ))
        );
    }

    #[test]
    fn rejects_code_challenge_with_wrong_length() {
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);

        for code_challenge in [
            // 42 characters
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-c",
            // 44 characters
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cMx",
        ] {
            let request = AuthorizationRequest::new(
                client_id,
                "https://example.com/callback".to_string(),
                "code".to_string(),
                "read".to_string(),
                "some-state".to_string(),
                code_challenge.to_string(),
                "S256".to_string(),
            );

            assert_eq!(
                request.validate_against_client(&client),
                Err(ValidationError::redirectable(
                    RedirectableValidationError::InvalidCodeChallenge,
                    "https://example.com/callback".to_string(),
                    "some-state".to_string(),
                )),
                "code_challenge: {code_challenge}"
            );
        }
    }

    #[test]
    fn rejects_code_challenge_with_non_base64url_characters() {
        let client_id = Uuid::new_v4();
        let client = make_client(client_id);

        for code_challenge in [
            // padding
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-c=",
            // standard base64 alphabet
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw+cM",
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw/cM",
            // allowed in a code_verifier, but not in an S256 challenge
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw.cM",
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw~cM",
        ] {
            let request = AuthorizationRequest::new(
                client_id,
                "https://example.com/callback".to_string(),
                "code".to_string(),
                "read".to_string(),
                "some-state".to_string(),
                code_challenge.to_string(),
                "S256".to_string(),
            );

            assert_eq!(
                request.validate_against_client(&client),
                Err(ValidationError::redirectable(
                    RedirectableValidationError::InvalidCodeChallenge,
                    "https://example.com/callback".to_string(),
                    "some-state".to_string(),
                )),
                "code_challenge: {code_challenge}"
            );
        }
    }
}
