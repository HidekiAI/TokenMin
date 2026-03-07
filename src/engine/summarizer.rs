use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    pub fn new(url: String, model: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60)) // Summarization can take time
                .build()
                .unwrap(),
            url,
            model,
        }
    }

    pub async fn summarize(&self, text: &str) -> Result<String, String> {
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

        let response = self.client
            .post(format!("{}/api/generate", self.url))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Ollama API returned error: {}", response.status()));
        }

        let body: OllamaResponse = response
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

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
        
        let _m = server.mock("POST", "/api/generate")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"response": "This is a summary."}"#)
            .create_async().await;

        let summarizer = Summarizer::new(url, "test-model".into());
        let result = summarizer.summarize("Original text").await;

        assert_eq!(result.unwrap(), "This is a summary.");
    }
}
