use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

/// The shared `error.html` page.
#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorPage<'a> {
    pub error: &'a str,
    pub error_description: &'a str,
}

/// Renders `template` as an HTML response with `status_code`, or a bare 500 when rendering fails.
pub fn render_html(template: &impl Template, status_code: StatusCode) -> Response {
    match template.render() {
        Ok(html) => (status_code, Html(html)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Renders the error page with `status_code`.
pub fn render_error_page(
    status_code: StatusCode,
    error: &str,
    error_description: &str,
) -> Response {
    render_html(
        &ErrorPage {
            error,
            error_description,
        },
        status_code,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn renders_the_error_page_with_the_status_code() {
        let response =
            render_error_page(StatusCode::BAD_REQUEST, "invalid_request", "Bad <input>.");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("<h1>invalid_request</h1>"), "{body}");
        assert!(body.contains("Bad &#60;input&#62;."), "{body}");
    }
}
