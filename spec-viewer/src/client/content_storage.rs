use async_trait::async_trait;
use reqwest::Client;

use crate::application::contract::content_gateway::{ContentGateway, GetContentError};

/// The content storage, reached over http.
pub struct HttpContentGateway {
    client: Client,
    url: String,
}

impl HttpContentGateway {
    pub fn new(url: &str) -> Self {
        HttpContentGateway {
            client: Client::new(),
            url: url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl ContentGateway for HttpContentGateway {
    async fn get(&self, uri: &str) -> Result<String, GetContentError> {
        let url = format!("{}/{}", self.url, uri.trim_start_matches('/'));

        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|error| GetContentError::Request(error.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(GetContentError::Status(status.as_u16(), url));
        }

        response
            .text()
            .await
            .map_err(|error| GetContentError::Request(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, http::StatusCode, routing::get};
    use tokio::net::TcpListener;

    async fn start_storage() -> HttpContentGateway {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/text.md", get(|| async { "# Title\n" }))
            .route(
                "/broken.md",
                get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
            );
        tokio::spawn(async move { axum::serve(listener, app).await });
        HttpContentGateway::new(&format!("http://{addr}"))
    }

    #[tokio::test]
    async fn returns_the_text() {
        let storage = start_storage().await;

        assert_eq!(storage.get("text.md").await.unwrap(), "# Title\n");
        assert_eq!(storage.get("/text.md").await.unwrap(), "# Title\n");
    }

    #[tokio::test]
    async fn maps_an_error_status() {
        let storage = start_storage().await;

        for (path, status) in [("missing.md", 404), ("broken.md", 500)] {
            match storage.get(path).await {
                Err(GetContentError::Status(actual, _)) => assert_eq!(actual, status),
                other => panic!("unexpected result: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn fails_to_connect_to_unreachable_storage() {
        assert!(matches!(
            HttpContentGateway::new("http://127.0.0.1:1")
                .get("text.md")
                .await,
            Err(GetContentError::Request(_))
        ));
    }
}
