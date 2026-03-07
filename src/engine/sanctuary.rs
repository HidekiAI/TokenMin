use regex::Regex;
use once_cell::sync::Lazy;

pub struct SanctuaryResult {
    pub sanitized_text: String,
    pub code_blocks: Vec<String>,
}

static CODE_BLOCK_RE: Lazy<Regex> = Lazy::new(|| {
    // Robust regex: allow optional whitespace after language tag
    Regex::new(r"(?s)```(\w+)?\s*\n(.*?)\n?```").unwrap()
});

pub fn extract_code_blocks(text: &str) -> SanctuaryResult {
    let mut code_blocks = Vec::new();
    let mut counter = 0;

    let sanitized_text = CODE_BLOCK_RE.replace_all(text, |caps: &regex::Captures| {
        let full_block = caps.get(0).unwrap().as_str().to_string();
        code_blocks.push(full_block);
        let replacement = format!("<<CODE_BLOCK_{}>>", counter);
        counter += 1;
        replacement
    }).to_string();

    SanctuaryResult {
        sanitized_text,
        code_blocks,
    }
}

pub fn restore_code_blocks(summary: &str, code_blocks: &[String]) -> String {
    let mut final_text = summary.to_string();
    let mut used_indices = std::collections::HashSet::new();

    for (i, block) in code_blocks.iter().enumerate() {
        let placeholder = format!("<<CODE_BLOCK_{}>>", i);
        if final_text.contains(&placeholder) {
            final_text = final_text.replace(&placeholder, block);
            used_indices.insert(i);
        }
    }

    // Safety fallback: append orphaned blocks if they were lost in summarization
    for (i, block) in code_blocks.iter().enumerate() {
        if !used_indices.contains(&i) {
            final_text.push_str("\n\n");
            final_text.push_str(block);
        }
    }

    final_text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanctuary_extraction() {
        let input = "Here is some code:\n```rust\nfn main() {}\n```\nAnd more:\n```python\nprint(1)\n```";
        let result = extract_code_blocks(input);
        
        assert!(result.sanitized_text.contains("<<CODE_BLOCK_0>>"));
        assert!(result.sanitized_text.contains("<<CODE_BLOCK_1>>"));
        assert_eq!(result.code_blocks.len(), 2);
        assert_eq!(result.code_blocks[0], "```rust\nfn main() {}\n```");
    }

    #[test]
    fn test_sanctuary_robust_extraction() {
        // Test with trailing space after lang tag
        let input = "```rust  \nfn main() {}\n```";
        let result = extract_code_blocks(input);
        assert_eq!(result.code_blocks.len(), 1);
        assert!(result.sanitized_text.contains("<<CODE_BLOCK_0>>"));
    }

    #[test]
    fn test_sanctuary_reassembly() {
        let summary = "Summary: <<CODE_BLOCK_0>> and <<CODE_BLOCK_1>>.";
        let code_blocks = vec![
            "```rust\nfn main() {}\n```".to_string(),
            "```python\nprint(1)\n```".to_string(),
        ];
        
        let final_text = restore_code_blocks(summary, &code_blocks);
        assert!(final_text.contains("fn main()"));
        assert!(final_text.contains("print(1)"));
    }

    #[test]
    fn test_sanctuary_orphaned_blocks() {
        let summary = "The summary lost the placeholders.";
        let code_blocks = vec!["```rust\nfn main() {}\n```".to_string()];
        
        let final_text = restore_code_blocks(summary, &code_blocks);
        assert!(final_text.contains("The summary lost the placeholders."));
        assert!(final_text.contains("```rust\nfn main() {}\n```"));
    }
}
