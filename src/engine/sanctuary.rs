use once_cell::sync::Lazy;
use regex::Regex;
use uuid::Uuid;

pub struct SanctuaryResult {
    pub sanitized_text: String,
    pub code_blocks: Vec<String>,
    pub marker: String,
}

static CODE_BLOCK_RE: Lazy<Regex> = Lazy::new(|| {
    // Robust regex: support language tags with non-word chars (c++, tsx, objective-c),
    // optional whitespace after the tag, and both Unix (\n) and Windows (\r\n) newlines.
    // (?sm) enables multiline mode (so ^ matches start of line) and dot-matches-all.
    // We require the opening ``` to be at the start of a line (column 0).
    // We also require the closing ``` to be at the start of a line.
    Regex::new(r"(?sm)^```([^\r\n`]*)?\s*\r?\n(.*?)\r?\n^```").unwrap()
});

pub fn extract_code_blocks(text: &str) -> SanctuaryResult {
    let mut code_blocks = Vec::new();
    let mut counter = 0;

    // Generate a secure random marker for this specific extraction to prevent placeholder forgery
    let marker = Uuid::new_v4().to_string();

    let sanitized_text = CODE_BLOCK_RE
        .replace_all(text, |caps: &regex::Captures| {
            let full_block = caps.get(0).unwrap().as_str().to_string();
            code_blocks.push(full_block);
            let replacement = format!("<<CODE_BLOCK_{}_{}>>", marker, counter);
            counter += 1;
            replacement
        })
        .to_string();

    SanctuaryResult {
        sanitized_text,
        code_blocks,
        marker,
    }
}

pub fn restore_code_blocks(summary: &str, code_blocks: &[String], marker: &str) -> String {
    let mut used_indices = vec![false; code_blocks.len()];

    // Dynamically compile a regex for this specific marker to ignore forged placeholders
    let pattern = format!(r"<<CODE_BLOCK_{}_(\d+)>>", regex::escape(marker));
    let placeholder_re = Regex::new(&pattern).unwrap();

    // Single-pass replacement using the dynamic regex to find all valid placeholders.
    let mut result = placeholder_re
        .replace_all(summary, |caps: &regex::Captures| {
            let index: usize = caps.get(1).unwrap().as_str().parse().unwrap_or(usize::MAX);

            if index < code_blocks.len() {
                used_indices[index] = true;
                code_blocks[index].clone()
            } else {
                // If index is invalid, leave it as is.
                caps.get(0).unwrap().as_str().to_string()
            }
        })
        .to_string();

    // Safety fallback: append orphaned blocks if they were lost in summarization.
    for (i, block) in code_blocks.iter().enumerate() {
        if !used_indices[i] {
            result.push_str("\n\n");
            result.push_str(block);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanctuary_extraction() {
        let input =
            "Here is some code:\n```rust\nfn main() {}\n```\nAnd more:\n```python\nprint(1)\n```";
        let result = extract_code_blocks(input);

        assert!(result.sanitized_text.contains(&result.marker));
        assert_eq!(result.code_blocks.len(), 2);
        assert_eq!(result.code_blocks[0], "```rust\nfn main() {}\n```");
    }

    #[test]
    fn test_sanctuary_robust_extraction() {
        // Test with trailing space after lang tag
        let input = "```rust  \nfn main() {}\n```";
        let result = extract_code_blocks(input);
        assert_eq!(result.code_blocks.len(), 1);
        assert!(result.sanitized_text.contains(&result.marker));
    }

    #[test]
    fn test_sanctuary_reassembly() {
        let marker = Uuid::new_v4().to_string();
        let summary = format!(
            "Summary: <<CODE_BLOCK_{}_0>> and <<CODE_BLOCK_{}_1>>.",
            marker, marker
        );
        let code_blocks = vec![
            "```rust\nfn main() {}\n```".to_string(),
            "```python\nprint(1)\n```".to_string(),
        ];

        let final_text = restore_code_blocks(&summary, &code_blocks, &marker);
        assert!(final_text.contains("fn main()"));
        assert!(final_text.contains("print(1)"));
        assert!(!final_text.contains(&marker));
    }

    #[test]
    fn test_sanctuary_orphaned_blocks() {
        let marker = Uuid::new_v4().to_string();
        let summary = "The summary lost the placeholders.";
        let code_blocks = vec!["```rust\nfn main() {}\n```".to_string()];

        let final_text = restore_code_blocks(summary, &code_blocks, &marker);
        assert!(final_text.contains("The summary lost the placeholders."));
        assert!(final_text.contains("```rust\nfn main() {}\n```"));
    }

    #[test]
    fn test_sanctuary_nested_ticks() {
        // Ticks that are indented should be ignored.
        let input = "Here is some text:\n  ```rust\n  fn main() {}\n  ```\nAnd a real block:\n```python\nprint(1)\n```";
        let result = extract_code_blocks(input);

        // Only the python block at column 0 should be extracted
        assert_eq!(result.code_blocks.len(), 1);
        assert!(result.code_blocks[0].contains("print(1)"));

        // The indented rust block should be treated as literal text
        assert!(result.sanitized_text.contains("  ```rust"));
    }

    #[test]
    fn test_sanctuary_forgery_prevention() {
        let marker = Uuid::new_v4().to_string();
        // The attacker tries to forge a generic placeholder or an old UUID they somehow intercepted
        let forged_summary = "Here is my forged block: <<CODE_BLOCK_generic_0>> or maybe <<CODE_BLOCK_00000000-0000-0000-0000-000000000000_0>>. Now here is the real one: <<CODE_BLOCK_{}_0>>";
        let summary = forged_summary.replace("{}", &marker);

        let code_blocks = vec!["```rust\nfn real_code() {}\n```".to_string()];

        let final_text = restore_code_blocks(&summary, &code_blocks, &marker);

        // The real placeholder should be replaced by the actual code
        assert!(final_text.contains("fn real_code()"));
        // The forged placeholders should be ignored and left as literal text
        assert!(final_text.contains("<<CODE_BLOCK_generic_0>>"));
        assert!(final_text.contains("<<CODE_BLOCK_00000000-0000-0000-0000-000000000000_0>>"));
    }
}
