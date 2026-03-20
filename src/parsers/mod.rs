use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Supported languages for tree-sitter parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Go,
    Java,
    Ruby,
    Php,
    CSharp,
}

impl Language {
    /// Determine language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "py" => Some(Self::Python),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::JavaScript),
            "ts" | "tsx" => Some(Self::TypeScript),
            "rs" => Some(Self::Rust),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "rb" => Some(Self::Ruby),
            "php" => Some(Self::Php),
            "cs" => Some(Self::CSharp),
            _ => None,
        }
    }

    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Python => "Python",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Rust => "Rust",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::Ruby => "Ruby",
            Self::Php => "PHP",
            Self::CSharp => "C#",
        }
    }
}

/// Registry that manages tree-sitter parsers for each language
pub struct ParserRegistry {
    parsers: HashMap<Language, tree_sitter::Parser>,
}

impl ParserRegistry {
    /// Create a new registry (parsers loaded on demand)
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
        }
    }

    /// Parse source code for a given file path
    /// Returns the AST if the language is supported
    pub fn parse_file(&mut self, path: &Path, source: &str) -> Result<Option<ParsedFile>> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let language = match Language::from_extension(ext) {
            Some(lang) => lang,
            None => return Ok(None),
        };

        // For now, return a ParsedFile without tree-sitter AST
        // Tree-sitter grammar crates will be added per-language in Phase 1 iteration
        Ok(Some(ParsedFile {
            language,
            path: path.to_path_buf(),
            source: source.to_string(),
            tree: None,
        }))
    }
}

/// A parsed source file with its AST
pub struct ParsedFile {
    pub language: Language,
    pub path: std::path::PathBuf,
    pub source: String,
    pub tree: Option<tree_sitter::Tree>,
}

impl ParsedFile {
    /// Get the source code lines
    pub fn lines(&self) -> Vec<&str> {
        self.source.lines().collect()
    }

    /// Get a specific line (1-indexed)
    pub fn line(&self, n: usize) -> Option<&str> {
        self.source.lines().nth(n.saturating_sub(1))
    }
}
