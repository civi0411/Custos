//! Structural Extraction for Context Compilation
//!
//! Extracts function signatures, types, enums, traits, and docstrings
//! to produce compact architectural outlines of source files (Custos.md §6.4 Step 2).

pub struct StructuralExtractor;

impl StructuralExtractor {
    /// Extracts structural outline from source code, omitting verbose implementation bodies.
    /// Supports Rust, Python, JavaScript, TypeScript, Go.
    pub fn extract_outline(source_code: &str, file_name: &str) -> String {
        let lines: Vec<&str> = source_code.lines().collect();
        if lines.len() <= 60 {
            // Keep full content if file is already compact
            return source_code.to_string();
        }

        let is_rust = file_name.ends_with(".rs");
        let is_python = file_name.ends_with(".py");
        let is_js_ts =
            file_name.ends_with(".js") || file_name.ends_with(".ts") || file_name.ends_with(".tsx");
        let is_go = file_name.ends_with(".go");

        let mut outline = Vec::new();
        let mut in_multiline_doc = false;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            // Doc comments
            if trimmed.starts_with("///") || trimmed.starts_with("//!") {
                outline.push(line);
                continue;
            }
            if trimmed.starts_with("/*")
                || trimmed.starts_with("/**")
                || trimmed.starts_with("\"\"\"")
            {
                in_multiline_doc = true;
                outline.push(line);
                if trimmed.ends_with("*/")
                    || (trimmed.len() > 3 && trimmed[3..].ends_with("\"\"\""))
                {
                    in_multiline_doc = false;
                }
                continue;
            }
            if in_multiline_doc {
                outline.push(line);
                if trimmed.ends_with("*/") || trimmed.ends_with("\"\"\"") {
                    in_multiline_doc = false;
                }
                continue;
            }

            // Rust declarations
            if is_rust {
                if trimmed.starts_with("pub fn ")
                    || trimmed.starts_with("fn ")
                    || trimmed.starts_with("pub async fn ")
                    || trimmed.starts_with("async fn ")
                    || trimmed.starts_with("pub struct ")
                    || trimmed.starts_with("struct ")
                    || trimmed.starts_with("pub enum ")
                    || trimmed.starts_with("enum ")
                    || trimmed.starts_with("pub trait ")
                    || trimmed.starts_with("trait ")
                    || trimmed.starts_with("pub type ")
                    || trimmed.starts_with("type ")
                    || trimmed.starts_with("impl ")
                    || trimmed.starts_with("#[derive")
                {
                    outline.push(line);
                }
                continue;
            }

            // Python declarations
            if is_python {
                if trimmed.starts_with("def ")
                    || trimmed.starts_with("async def ")
                    || trimmed.starts_with("class ")
                    || trimmed.starts_with("@")
                {
                    outline.push(line);
                }
                continue;
            }

            // JavaScript / TypeScript declarations
            if is_js_ts {
                if trimmed.starts_with("export function ")
                    || trimmed.starts_with("function ")
                    || trimmed.starts_with("export class ")
                    || trimmed.starts_with("class ")
                    || trimmed.starts_with("export interface ")
                    || trimmed.starts_with("interface ")
                    || trimmed.starts_with("export type ")
                    || trimmed.starts_with("type ")
                    || trimmed.starts_with("export const ")
                {
                    outline.push(line);
                }
                continue;
            }

            // Go declarations
            if is_go {
                if trimmed.starts_with("func ")
                    || trimmed.starts_with("type ")
                    || trimmed.starts_with("package ")
                {
                    outline.push(line);
                }
                continue;
            }

            // Fallback for other files: keep first 50 lines
            if outline.len() < 50 {
                outline.push(line);
            }
        }

        if outline.is_empty() {
            // Fallback: Return first 50 lines if no structural patterns matched
            source_code.lines().take(50).collect::<Vec<_>>().join("\n")
        } else {
            outline.join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_outline() {
        let mut rust_code = String::new();
        rust_code.push_str("/// Module docs\n");
        rust_code.push_str("pub struct WorkerConfig {\n    pub id: String,\n}\n");
        rust_code.push_str("impl WorkerConfig {\n");
        rust_code.push_str("    pub fn new() -> Self {\n");
        for i in 0..100 {
            rust_code.push_str(&format!("        let var_{i} = {i} * 2;\n"));
        }
        rust_code.push_str("        Self { id: \"test\".into() }\n    }\n}\n");

        let outline = StructuralExtractor::extract_outline(&rust_code, "config.rs");
        assert!(outline.contains("/// Module docs"));
        assert!(outline.contains("pub struct WorkerConfig"));
        assert!(outline.contains("pub fn new()"));
        // Deep loop body lines should not be preserved in full
        assert!(!outline.contains("let var_50 ="));
    }
}
