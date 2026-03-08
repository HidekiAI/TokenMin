pub mod sanctuary;
pub mod summarizer;

use crate::models::{Message, ProcessingStatus};
use std::collections::HashSet;

pub struct Engine {
    pub bypass_models: HashSet<String>,
}

impl Engine {
    pub fn new(bypass_models: Vec<String>) -> Self {
        let mut set = HashSet::new();
        for m in bypass_models {
            set.insert(m);
        }
        Self { bypass_models: set }
    }

    pub fn should_bypass(&self, message: &Message) -> bool {
        if let Some(model) = &message.model {
            return self.bypass_models.contains(model);
        }
        false
    }

    pub fn process_bypass(&self, mut message: Message) -> Message {
        message.processed_content = Some(std::mem::take(&mut message.raw_content));
        message.status = ProcessingStatus::Skipped;
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Message, ProcessingStatus};

    #[test]
    fn test_bypass_logic() {
        let engine = Engine::new(vec!["copilot-chat".to_string(), "gpt-3.5".to_string()]);

        let msg_to_bypass = Message {
            id: 1,
            session_id: "test".into(),
            role: "user".into(),
            raw_content: "Hello".into(),
            processed_content: None,
            status: ProcessingStatus::Pending,
            model: Some("copilot-chat".into()),
        };

        let msg_to_process = Message {
            id: 2,
            session_id: "test".into(),
            role: "user".into(),
            raw_content: "Hello".into(),
            processed_content: None,
            status: ProcessingStatus::Pending,
            model: Some("gpt-4".into()),
        };

        assert!(engine.should_bypass(&msg_to_bypass));
        assert!(!engine.should_bypass(&msg_to_process));

        let processed = engine.process_bypass(msg_to_bypass);
        assert_eq!(processed.status, ProcessingStatus::Skipped);
        assert_eq!(processed.processed_content.unwrap(), "Hello");
    }
}
