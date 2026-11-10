//! Turns a session's [`AuthProgress`] into what the sequence diagram shows. All formatting
//! happens here, so the templates only lay it out.

use serde::Serialize;

use crate::{
    config,
    security::{
        AuthProgress,
        auth::{
            check::Check,
            error::AuthError,
            par::par_url,
            token::{jwt_claims, jwt_header, token_url},
        },
    },
};

/// The lifelines of the diagram, left to right.
pub const ACTORS: [&str; 3] = ["Browser", "Example Client", "Auth Server"];
const BROWSER: usize = 0;
const CLIENT: usize = 1;
const AUTH_SERVER: usize = 2;

pub struct FlowView {
    pub steps: Vec<StepView>,
    pub token: Option<String>,
    pub id_token: Option<IdTokenView>,
}

/// The ID token, raw and decoded. Nothing is verified here, the checks happen in step 5.
pub struct IdTokenView {
    /// The three base64url parts: header, claims, signature.
    pub header_part: String,
    pub claims_part: String,
    pub signature_part: String,
    /// The decoded JOSE header, pretty-printed.
    pub header: String,
    pub claims: Vec<ClaimView>,
}

pub struct ClaimView {
    pub name: String,
    pub value: String,
    /// What the claim means (OpenID Connect Core 1.0 section 2), empty for unknown claims.
    pub meaning: &'static str,
    /// For timestamps, the date and how far away it is.
    pub note: Option<String>,
}

impl IdTokenView {
    fn new(id_token: &str) -> Self {
        let mut parts = id_token.splitn(3, '.');
        let mut next = || parts.next().unwrap_or_default().to_string();
        let (header_part, claims_part, signature_part) = (next(), next(), next());

        let header = jwt_header(id_token)
            .map(|header| pretty_json(&header))
            .unwrap_or_else(|| "<not decodable>".to_string());
        let claims = jwt_claims(id_token)
            .map(|claims| claims.into_iter().map(|(name, value)| ClaimView::new(name, value)).collect())
            .unwrap_or_default();

        Self { header_part, claims_part, signature_part, header, claims }
    }
}

impl ClaimView {
    fn new(name: String, value: serde_json::Value) -> Self {
        let meaning = claim_meaning(&name);
        let note = match (name.as_str(), value.as_i64()) {
            ("exp" | "iat" | "nbf" | "auth_time", Some(seconds)) => Some(describe_timestamp(seconds)),
            _ => None,
        };
        let value = match value {
            serde_json::Value::String(value) => value,
            other => other.to_string(),
        };
        Self { name, value, meaning, note }
    }
}

fn claim_meaning(name: &str) -> &'static str {
    match name {
        "iss" => "Issuer: the auth-server that issued the token",
        "sub" => "Subject: the user's id, never reassigned",
        "aud" => "Audience: the client_id it was issued to",
        "exp" => "Expires at",
        "iat" => "Issued at",
        "nbf" => "Not valid before",
        "auth_time" => "When the user logged in",
        "nonce" => "Echo of the authorization request's nonce",
        "azp" => "Authorized party",
        "acr" => "Authentication context class",
        "amr" => "Authentication methods",
        "at_hash" => "Hash of the access token",
        _ => "",
    }
}

/// `2026-11-10 14:03:12 UTC (in 4 min)` for a unix timestamp.
fn describe_timestamp(seconds: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(seconds);
    let delta = seconds - now;
    let relative = match delta.unsigned_abs() {
        0..=59 => format!("{} s", delta.unsigned_abs()),
        60..=3599 => format!("{} min", delta.unsigned_abs() / 60),
        3600..=86399 => format!("{} h", delta.unsigned_abs() / 3600),
        other => format!("{} d", other / 86400),
    };
    let relative = if delta >= 0 { format!("in {relative}") } else { format!("{relative} ago") };
    format!("{} ({relative})", format_utc(seconds))
}

/// Formats a unix timestamp as a UTC date and time, without pulling in a date crate.
fn format_utc(seconds: i64) -> String {
    let (days, secs) = (seconds.div_euclid(86400), seconds.rem_euclid(86400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

pub struct StepView {
    pub number: usize,
    pub title: &'static str,
    pub description: &'static str,
    pub status: Status,
    pub messages: Vec<MessageView>,
    /// Values the auth-server returned, compared against the expected ones.
    pub checks: Vec<CheckView>,
    pub action: Option<Action>,
}

pub struct CheckView {
    pub name: &'static str,
    pub expected: String,
    pub actual: String,
    pub passed: bool,
}

impl CheckView {
    fn new(check: &Check) -> Self {
        Self {
            name: check.name,
            expected: check.expected.clone(),
            actual: check.actual.clone().unwrap_or_else(|| "(missing)".to_string()),
            passed: check.passed(),
        }
    }

    pub fn class(&self) -> &'static str {
        if self.passed { "done" } else { "failed" }
    }

    pub fn mark(&self) -> &'static str {
        if self.passed { "✓" } else { "✗" }
    }
}

fn check_views(checks: &[Check]) -> Vec<CheckView> {
    checks.iter().map(CheckView::new).collect()
}

/// The checks as plain text, the body of the arrow the client sends to itself.
fn checks_body(checks: &[CheckView]) -> String {
    checks
        .iter()
        .map(|check| {
            format!(
                "{} {}\n  expected: {}\n  received: {}",
                check.mark(),
                check.name,
                check.expected,
                check.actual
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Done,
    Current,
    Pending,
    Failed,
}

impl Status {
    pub fn class(&self) -> &'static str {
        match self {
            Status::Done => "done",
            Status::Current => "current",
            Status::Pending => "pending",
            Status::Failed => "failed",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Status::Done => "done",
            Status::Current => "next",
            Status::Pending => "pending",
            Status::Failed => "failed",
        }
    }
}

pub struct Action {
    pub label: &'static str,
    /// Posted to with htmx, the re-rendered diagram replaces the current one.
    pub post_to: Option<String>,
    /// A plain link instead, the browser leaves the page.
    pub href: Option<String>,
}

impl Action {
    fn post(path: &str, label: &'static str) -> Self {
        Self { label, post_to: Some(format!("/flow/{path}")), href: None }
    }

    fn navigate(url: String, label: &'static str) -> Self {
        Self { label, post_to: None, href: Some(url) }
    }
}

/// One arrow in the diagram.
pub struct MessageView {
    pub from: usize,
    pub to: usize,
    pub label: String,
    /// The full request or response, shown when the arrow is expanded. `None` while pending.
    pub body: Option<String>,
    pub status: Status,
}

impl MessageView {
    fn new(from: usize, to: usize, label: impl Into<String>, body: Option<String>, status: Status) -> Self {
        Self { from, to, label: label.into(), body, status }
    }

    pub fn is_self(&self) -> bool {
        self.from == self.to
    }

    /// `ltr`, `rtl` or `self`, the arrow's direction.
    pub fn direction(&self) -> &'static str {
        match self.from.cmp(&self.to) {
            std::cmp::Ordering::Less => "ltr",
            std::cmp::Ordering::Greater => "rtl",
            std::cmp::Ordering::Equal => "self",
        }
    }

    /// The first grid column (1-based) the arrow covers.
    pub fn column_start(&self) -> usize {
        self.from.min(self.to) + 1
    }

    /// The number of lifeline columns the arrow covers.
    pub fn span(&self) -> usize {
        self.from.abs_diff(self.to) + 1
    }
}

impl FlowView {
    pub fn new(progress: &AuthProgress) -> Self {
        let mut steps = vec![
            prepare_step(progress),
            par_step(progress),
            authorize_step(progress),
            callback_step(progress),
            token_step(progress),
        ];

        // The first step that isn't done (and didn't fail) is the one to run next.
        let mut blocked = false;
        for step in &mut steps {
            if blocked {
                step.status = Status::Pending;
            } else if step.status != Status::Done {
                if step.status == Status::Pending {
                    step.status = Status::Current;
                }
                blocked = true;
            }
        }
        for step in &mut steps {
            if !matches!(step.status, Status::Current | Status::Failed) {
                step.action = None;
            }
            for message in &mut step.messages {
                if step.status == Status::Pending {
                    message.status = Status::Pending;
                }
            }
        }

        Self {
            steps,
            token: progress.token.clone(),
            id_token: progress.id_token.as_deref().map(IdTokenView::new),
        }
    }
}

fn result_status<T>(result: &Option<Result<T, AuthError>>) -> Status {
    match result {
        Some(Ok(_)) => Status::Done,
        Some(Err(_)) => Status::Failed,
        None => Status::Pending,
    }
}

fn prepare_step(progress: &AuthProgress) -> StepView {
    let body = match (&progress.state, &progress.code_verifier) {
        (Some(state), Some(verifier)) => Some(pretty_json(&serde_json::json!({
            "state": state.value(),
            "code_verifier": verifier.value(),
            "code_challenge": verifier.code_challenge(),
            "code_challenge_method": verifier.code_challenge_method(),
        }))),
        _ => None,
    };
    let status = if body.is_some() { Status::Done } else { Status::Pending };

    StepView {
        number: 1,
        title: "Generate state and PKCE verifier",
        description: "A random state ties the callback to this request; the verifier's SHA-256 challenge goes out now, the verifier itself only with the token request (RFC 7636).",
        status,
        messages: vec![MessageView::new(CLIENT, CLIENT, "state + code_verifier", body, status)],
        checks: Vec::new(),
        action: Some(Action::post("prepare", "Generate")),
    }
}

fn par_step(progress: &AuthProgress) -> StepView {
    let status = result_status(&progress.par_response);
    let request = progress
        .par_request
        .as_ref()
        .map(|request| form_request(&par_url(), request));
    let (response_label, response) = match &progress.par_response {
        Some(Ok(response)) => ("201 Created".to_string(), Some(pretty_json(response))),
        Some(Err(error)) => (error_label(error), Some(error.to_string())),
        None => ("request_uri".to_string(), None),
    };

    StepView {
        number: 2,
        title: "Push the authorization request",
        description: "The client sends the authorization parameters straight to the auth-server (RFC 9126) and gets back a request_uri referencing them, so nothing sensitive travels through the browser.",
        status,
        messages: vec![
            MessageView::new(CLIENT, AUTH_SERVER, "POST /par", request, status),
            MessageView::new(AUTH_SERVER, CLIENT, response_label, response, status),
        ],
        checks: Vec::new(),
        action: Some(Action::post("par", "Push request")),
    }
}

fn authorize_step(progress: &AuthProgress) -> StepView {
    let status = if progress.callback_query.is_some() { Status::Done } else { Status::Pending };
    let request = progress.auth_redirect_uri.as_ref().map(|url| format!("GET {url}"));
    let response = progress.callback_query.as_ref().map(|query| {
        format!("303 See Other\nLocation: {}", callback_url(query))
    });

    StepView {
        number: 3,
        title: "Log in and authorize",
        description: "The browser goes to the auth-server with only client_id and request_uri. The user logs in and confirms, then the auth-server redirects back to the client.",
        status,
        messages: vec![
            MessageView::new(BROWSER, AUTH_SERVER, "GET /authorize", request, status),
            MessageView::new(AUTH_SERVER, BROWSER, "303 → redirect_uri", response, status),
        ],
        checks: Vec::new(),
        action: progress
            .auth_redirect_uri
            .clone()
            .map(|url| Action::navigate(url, "Authorize")),
    }
}

fn callback_step(progress: &AuthProgress) -> StepView {
    let status = result_status(&progress.auth_response);
    let request = progress
        .callback_query
        .as_ref()
        .map(|query| format!("GET {}", callback_url(query)));
    let checks = check_views(&progress.callback_checks);
    let checks_passed = checks.iter().all(|check| check.passed);
    let check_label = match &progress.auth_response {
        None => "check state + iss",
        Some(_) if checks_passed => "state + iss ok",
        Some(_) => "state / iss mismatch",
    };
    let check = progress.auth_response.as_ref().map(|response| {
        let outcome = match response {
            Ok(response) => pretty_json(response),
            Err(error) => format!("rejected: {error}"),
        };
        format!("{}\n\n{outcome}", checks_body(&checks))
    });

    StepView {
        number: 4,
        title: "Receive the callback",
        description: "The client checks that state is the one it sent and iss is the auth-server it asked (RFC 9207) before trusting the code.",
        status,
        messages: vec![
            MessageView::new(BROWSER, CLIENT, "GET /?code=…", request, status),
            MessageView::new(CLIENT, CLIENT, check_label, check, status),
        ],
        checks,
        // After a rejected callback, authorizing again is the way forward.
        action: progress
            .auth_redirect_uri
            .clone()
            .map(|url| Action::navigate(url, "Authorize again")),
    }
}

fn token_step(progress: &AuthProgress) -> StepView {
    let status = result_status(&progress.token_response);
    let request = progress
        .token_request
        .as_ref()
        .map(|request| form_request(&token_url(), request));
    let checks = check_views(&progress.token_checks);
    // With checks, the auth-server answered 200; a failure is then the client rejecting the token.
    let (response_label, response) = match &progress.token_response {
        Some(Ok(response)) => ("200 OK".to_string(), Some(pretty_json(response))),
        Some(Err(_)) if !checks.is_empty() => ("200 OK".to_string(), None),
        Some(Err(error)) => (error_label(error), Some(error.to_string())),
        None => ("access_token".to_string(), None),
    };
    let (check_label, check) = match &progress.token_response {
        Some(_) if !checks.is_empty() => {
            let label = if checks.iter().all(|check| check.passed) {
                "id_token iss ok"
            } else {
                "id_token iss mismatch"
            };
            let mut body = checks_body(&checks);
            if let Some(Err(error)) = &progress.token_response {
                body.push_str(&format!("\n\nrejected: {error}"));
            }
            (label, Some(body))
        }
        _ => ("check id_token iss", None),
    };
    let response_status = if checks.is_empty() { status } else { Status::Done };

    StepView {
        number: 5,
        title: "Exchange the code for tokens",
        description: "The client sends the code with the PKCE verifier; the auth-server hashes the verifier, compares it to the challenge from step 2 and issues the tokens. The client checks the ID token's iss is the auth-server it asked.",
        status,
        messages: vec![
            MessageView::new(CLIENT, AUTH_SERVER, "POST /token", request, status),
            MessageView::new(AUTH_SERVER, CLIENT, response_label, response, response_status),
            MessageView::new(CLIENT, CLIENT, check_label, check, status),
        ],
        checks,
        action: Some(Action::post("token", "Exchange code")),
    }
}

fn pretty_json(value: &impl Serialize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|error| format!("<not serializable: {error}>"))
}

/// A form POST as it goes over the wire, one parameter per line.
fn form_request(url: &str, form: &impl Serialize) -> String {
    let body = serde_urlencoded::to_string(form)
        .unwrap_or_else(|error| format!("<not encodable: {error}>"))
        .replace('&', "\n&");
    format!("POST {url}\nContent-Type: application/x-www-form-urlencoded\n\n{body}")
}

fn callback_url(query: &impl Serialize) -> String {
    let query = serde_urlencoded::to_string(query).unwrap_or_default();
    format!("{}/?{query}", config::home_url())
}

fn error_label(error: &AuthError) -> String {
    match error {
        AuthError::Rejected { status_code, error, .. } => format!("{status_code} {error}"),
        AuthError::UnexpectedResponse { status_code } => format!("{status_code}"),
        _ => "request failed".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_unix_timestamps_as_utc() {
        assert_eq!(format_utc(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(format_utc(951_782_400), "2000-02-29 00:00:00 UTC");
        assert_eq!(format_utc(1_794_323_045), "2026-11-10 15:04:05 UTC");
    }
}
