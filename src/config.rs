use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub bypass_models: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();

        let db_path =
            env::var("TOKENMIN_DB").unwrap_or_else(|_| "/dev/shm/tokenmin/queue.db".to_string());

        let ollama_url =
            env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());

        let ollama_model =
            env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:7b".to_string());

        let bypass_models = env::var("BYPASS_MODELS")
            .unwrap_or_else(|_| "".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            db_path,
            ollama_url,
            ollama_model,
            bypass_models,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    // Use a global mutex to serialize environment variable access in tests
    static ENV_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn test_config_loading() {
        let _guard = ENV_MUTEX.lock().unwrap();

        unsafe {
            env::set_var("TOKENMIN_DB", "/tmp/test.db");
            env::set_var("BYPASS_MODELS", "model1, model2 ");
        }

        let config = Config::from_env();
        assert_eq!(config.db_path, "/tmp/test.db");
        assert_eq!(config.bypass_models, vec!["model1", "model2"]);
    }
}
