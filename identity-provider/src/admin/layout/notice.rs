use askama::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::admin::layout::html::{html_response, render_html};

/// The `#notice` element.
#[derive(Template)]
#[template(path = "layout/notice.html")]
pub struct Notice {
    pub message: String,
    pub temporary_password: Option<String>,
    pub is_error: bool,
    /// Swapped out-of-band next to the main content of the response.
    pub oob: bool,
}

impl Notice {
    /// A success message, swapped out-of-band.
    pub fn success(message: String) -> Self {
        Self {
            message,
            temporary_password: None,
            is_error: false,
            oob: true,
        }
    }

    /// A success message showing a temporary password, swapped out-of-band.
    pub fn temporary_password(message: String, temporary_password: String) -> Self {
        Self {
            temporary_password: Some(temporary_password),
            ..Self::success(message)
        }
    }

    /// An error message, swapped as the main content.
    pub fn error(message: String) -> Self {
        Self {
            message,
            temporary_password: None,
            is_error: true,
            oob: false,
        }
    }
}

html_response!(Notice);

/// An error shown in the `#notice` element, whatever the request targeted.
#[derive(Debug)]
pub struct ErrorNotice {
    status: StatusCode,
    message: &'static str,
}

impl ErrorNotice {
    pub fn new(status: StatusCode, message: &'static str) -> Self {
        Self { status, message }
    }

    pub fn invalid_form() -> Self {
        Self::new(StatusCode::BAD_REQUEST, "The form is invalid.")
    }

    pub fn invalid_user_id() -> Self {
        Self::new(StatusCode::BAD_REQUEST, "The user ID is invalid.")
    }

    pub fn invalid_user_name() -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "The user name may only contain letters, digits and dots.",
        )
    }

    pub fn user_name_already_exists() -> Self {
        Self::new(StatusCode::CONFLICT, "The user name already exists.")
    }

    pub fn user_not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "The user was not found.")
    }

    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "An internal server error occurred.",
        )
    }
}

impl IntoResponse for ErrorNotice {
    fn into_response(self) -> Response {
        let mut response = render_html(&Notice::error(self.message.to_string()), self.status);
        let headers = response.headers_mut();
        headers.insert("HX-Retarget", "#notice".parse().unwrap());
        headers.insert("HX-Reswap", "outerHTML".parse().unwrap());
        response
    }
}
