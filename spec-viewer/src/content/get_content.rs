use std::sync::LazyLock;

use reqwest::{Client, StatusCode};

use crate::config::Config;

/// Shared so the connections to the content storage are reused.
static CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

#[derive(Debug, thiserror::Error)]
pub enum GetContentError {
    #[error("failed to fetch content: {0}")]
    Request(#[source] reqwest::Error),
    #[error("content storage {1} answered with {0}")]
    Status(StatusCode, String),
}

/// Fetches the full text behind the `content_uri` of a patch or proposal.
pub async fn get_content(uri: &str) -> Result<String, GetContentError> {
    fetch(Config::content_storage_url(), uri).await
}

/// Fetches `uri` from the content storage at `storage_url`.
async fn fetch(storage_url: &str, uri: &str) -> Result<String, GetContentError> {
    let url = format!("{storage_url}/{}", uri.trim_start_matches('/'));

    let response = CLIENT
        .get(url.clone())
        .send()
        .await
        .map_err(GetContentError::Request)?;

    let status = response.status();
    if !status.is_success() {
        return Err(GetContentError::Status(status, url));
    }

    response.text().await.map_err(GetContentError::Request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, http::StatusCode as AxumStatus, routing::get};
    use tokio::net::TcpListener;

    async fn start_storage() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/text.md", get(|| async { "# Title\n" }))
            .route(
                "/broken.md",
                get(|| async { AxumStatus::INTERNAL_SERVER_ERROR }),
            );
        tokio::spawn(async move { axum::serve(listener, app).await });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn returns_the_text() {
        let url = start_storage().await;

        assert_eq!(fetch(&url, "text.md").await.unwrap(), "# Title\n");
        assert_eq!(fetch(&url, "/text.md").await.unwrap(), "# Title\n");
    }

    #[tokio::test]
    async fn maps_an_error_status() {
        let url = start_storage().await;

        for (path, status) in [
            ("missing.md", StatusCode::NOT_FOUND),
            ("broken.md", StatusCode::INTERNAL_SERVER_ERROR),
        ] {
            match fetch(&url, path).await {
                Err(GetContentError::Status(actual, _)) => assert_eq!(actual, status),
                other => panic!("unexpected result: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn fails_to_connect_to_unreachable_storage() {
        assert!(matches!(
            fetch("http://127.0.0.1:1", "text.md").await,
            Err(GetContentError::Request(_))
        ));
    }
}
