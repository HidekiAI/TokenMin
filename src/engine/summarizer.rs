use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

#[derive(Debug)]
pub enum SummarizerError {
    Network(reqwest::Error),
    Api(reqwest::StatusCode),
    Parse(reqwest::Error),
}

impl fmt::Display for SummarizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(e) => write!(f, "Network error: {}", e),
            Self::Api(s) => write!(f, "Ollama API returned error status: {}", s),
            Self::Parse(e) => write!(f, "Failed to parse Ollama response: {}", e),
        }
    }
}

impl std::error::Error for SummarizerError {}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

pub struct Summarizer {
    client: Client,
    url: String,
    model: String,
}

impl Summarizer {
    pub fn new(url: String, model: String) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60)) // Summarization can take time
            .build()?;

        // Normalize URL: trim trailing slash to avoid double-slash in endpoint construction
        let url = url.trim_end_matches('/').to_string();

        Ok(Self { client, url, model })
    }

    pub async fn summarize(&self, text: &str) -> Result<String, SummarizerError> {
        // Use clear delimiters to mitigate prompt injection
        let prompt = format!(
            "Summarize the following text concisely, retaining all key technical constraints and request details. \
             Do not output conversational filler. \n\n\
             [INPUT_START]\n{}\n[INPUT_END]",
            text
        );

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
        };

        let endpoint = format!("{}/api/generate", self.url);
        let response = self
            .client
            .post(endpoint)
            .json(&request)
            .send()
            .await
            .map_err(SummarizerError::Network)?;

        if !response.status().is_success() {
            return Err(SummarizerError::Api(response.status()));
        }

        let body: OllamaResponse = response.json().await.map_err(SummarizerError::Parse)?;

        Ok(body.response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_summarizer_client() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let _m = server
            .mock("POST", "/api/generate")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"response": "This is a summary."}"#)
            .create_async()
            .await;

        let summarizer = Summarizer::new(url, "test-model".into()).unwrap();
        let result = summarizer.summarize("Original text").await;

        assert_eq!(result.unwrap(), "This is a summary.");
    }
}
