use crate::engine::Engine;
use crate::engine::sanctuary::{extract_code_blocks, restore_code_blocks};
use crate::engine::summarizer::Summarizer;
use crate::models::Message;
use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

#[wasm_bindgen]
pub struct TokenMinWasm {
    engine: Engine,
    summarizer_url: String,
    summarizer_model: String,
}

#[wasm_bindgen]
impl TokenMinWasm {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::boxed_local)]
    pub fn new(
        bypass_models: Box<[JsValue]>,
        summarizer_url: String,
        summarizer_model: String,
    ) -> Result<TokenMinWasm, JsValue> {
        let mut bypass = Vec::new();
        for val in bypass_models.iter() {
            if let Some(s) = val.as_string() {
                bypass.push(s);
            }
        }

        Ok(Self {
            engine: Engine::new(bypass),
            summarizer_url,
            summarizer_model,
        })
    }

    #[wasm_bindgen]
    pub fn compress(&self, raw_content: String, model: Option<String>) -> Promise {
        let url = self.summarizer_url.clone();
        let summarizer_model = self.summarizer_model.clone();
        let engine = self.engine.clone();

        future_to_promise(async move {
            let msg = Message {
                id: 0,
                session_id: "wasm".into(),
                role: "user".into(),
                raw_content: raw_content.clone(),
                processed_content: None,
                status: crate::models::ProcessingStatus::Pending,
                model: model.clone(),
                hmac_signature: "".into(),
            };

            if engine.should_bypass(&msg) {
                return Ok(JsValue::from_str(&raw_content));
            }

            let summarizer = Summarizer::new(url, summarizer_model)
                .map_err(|e| JsValue::from_str(&format!("Summarizer init error: {}", e)))?;

            let sanctuary = extract_code_blocks(&raw_content);

            match summarizer.summarize(&sanctuary.sanitized_text).await {
                Ok(summary) => {
                    let final_content =
                        restore_code_blocks(&summary, &sanctuary.code_blocks, &sanctuary.marker);
                    Ok(JsValue::from_str(&final_content))
                }
                Err(_e) => {
                    // Fallback to raw if summarization fails
                    Ok(JsValue::from_str(&raw_content))
                }
            }
        })
    }
}
