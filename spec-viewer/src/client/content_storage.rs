use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use crate::application::contract::content_gateway::{ContentGateway, GetContentError};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
/// For the whole request, including reading the body.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// The largest text accepted. Anyone can write a proposal, and a text is rendered or diffed
/// in memory.
const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;

/// The content storage, reached over http.
pub struct HttpContentGateway {
    client: Client,
    url: String,
    max_text_bytes: usize,
}

impl HttpContentGateway {
    pub fn new(url: &str) -> Self {
        Self::with_limits(url, REQUEST_TIMEOUT, MAX_TEXT_BYTES)
    }

    fn with_limits(url: &str, request_timeout: Duration, max_text_bytes: usize) -> Self {
        HttpContentGateway {
            client: Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(request_timeout)
                .build()
                .expect("the http client is configured with valid settings"),
            url: url.trim_end_matches('/').to_string(),
            max_text_bytes,
        }
    }
}

#[async_trait]
impl ContentGateway for HttpContentGateway {
    async fn get(&self, uri: &str) -> Result<String, GetContentError> {
        let url = format!("{}/{}", self.url, uri.trim_start_matches('/'));
        let too_large = || GetContentError::TooLarge(url.clone(), self.max_text_bytes);

        let mut response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|error| GetContentError::Request(error.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(GetContentError::Status(status.as_u16(), url));
        }

        // The declared size is only a hint, so the body is counted while it is read, too.
        if response
            .content_length()
            .is_some_and(|length| length > self.max_text_bytes as u64)
        {
            return Err(too_large());
        }
        let mut text = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| GetContentError::Request(error.to_string()))?
        {
            if text.len() + chunk.len() > self.max_text_bytes {
                return Err(too_large());
            }
            text.extend_from_slice(&chunk);
        }

        Ok(String::from_utf8_lossy(&text).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::StatusCode, routing::get};
    use tokio::net::TcpListener;

    async fn start_storage_with(
        request_timeout: Duration,
        max_text_bytes: usize,
    ) -> HttpContentGateway {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/text.md", get(|| async { "# Title\n" }))
            .route(
                "/broken.md",
                get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
            )
            .route("/big.md", get(|| async { "x".repeat(100) }))
            .route(
                "/streamed.md",
                get(|| async {
                    // Without a content length, the size is only known after reading.
                    let chunks = (0..10).map(|_| Ok::<_, std::io::Error>("x".repeat(10)));
                    Body::from_stream(tokio_stream::iter(chunks))
                }),
            )
            .route(
                "/slow.md",
                get(|| async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    "late"
                }),
            );
        tokio::spawn(async move { axum::serve(listener, app).await });
        HttpContentGateway::with_limits(&format!("http://{addr}"), request_timeout, max_text_bytes)
    }

    async fn start_storage() -> HttpContentGateway {
        start_storage_with(REQUEST_TIMEOUT, MAX_TEXT_BYTES).await
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

    #[tokio::test]
    async fn rejects_a_text_declared_larger_than_the_limit() {
        let storage = start_storage_with(REQUEST_TIMEOUT, 99).await;

        assert!(matches!(
            storage.get("big.md").await,
            Err(GetContentError::TooLarge(_, 99))
        ));
    }

    #[tokio::test]
    async fn rejects_a_streamed_text_larger_than_the_limit() {
        let storage = start_storage_with(REQUEST_TIMEOUT, 99).await;

        assert!(matches!(
            storage.get("streamed.md").await,
            Err(GetContentError::TooLarge(_, 99))
        ));
        assert_eq!(
            start_storage_with(REQUEST_TIMEOUT, 100)
                .await
                .get("streamed.md")
                .await
                .unwrap()
                .len(),
            100
        );
    }

    #[tokio::test]
    async fn gives_up_on_a_slow_storage() {
        let storage = start_storage_with(Duration::from_millis(100), MAX_TEXT_BYTES).await;

        assert!(matches!(
            storage.get("slow.md").await,
            Err(GetContentError::Request(_))
        ));
    }
}
