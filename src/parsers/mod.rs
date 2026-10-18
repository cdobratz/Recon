//! Tree-sitter parsing for Python, JavaScript, TypeScript, Go, and Rust.
//!
//! Query files live in `queries/<language>.scm`. Languages without a grammar
//! stay on the regex scanners.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language as TsLanguage, Node, Parser, Query, QueryCursor, Tree};

/// Supported languages for tree-sitter parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Rust,
    Go,
    Java,
    Ruby,
    Php,
    CSharp,
}

impl Language {
    /// Determine language from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "py" => Some(Self::Python),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::JavaScript),
            "ts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "rs" => Some(Self::Rust),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "rb" => Some(Self::Ruby),
            "php" => Some(Self::Php),
            "cs" => Some(Self::CSharp),
            _ => None,
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }

    /// Display name used by the regex scanners.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Python => "Python",
            Self::JavaScript => "JavaScript",
            Self::TypeScript | Self::Tsx => "TypeScript",
            Self::Rust => "Rust",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::Ruby => "Ruby",
            Self::Php => "PHP",
            Self::CSharp => "C#",
        }
    }

    pub fn has_grammar(&self) -> bool {
        matches!(
            self,
            Self::Python | Self::JavaScript | Self::TypeScript | Self::Tsx | Self::Rust | Self::Go
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstHitKind {
    Function,
    Endpoint,
    ShellCall,
    DatabaseCall,
}

/// One node selected by a language query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstHit {
    pub kind: AstHitKind,
    pub line: usize,
    pub snippet: String,
    pub name: String,
    pub detail: String,
}

/// A parsed source file. `parsed` is false when the grammar failed or does not exist.
pub struct ParsedFile {
    pub language: Language,
    pub hits: Vec<AstHit>,
    pub parsed: bool,
}

/// Registry that manages tree-sitter parsers for each language.
pub struct ParserRegistry {
    parsers: HashMap<Language, Parser>,
    queries: HashMap<Language, Query>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
            queries: HashMap::new(),
        }
    }

    /// Parse `source`. `Ok(None)` means the extension is not a known language.
    pub fn parse_file(&mut self, path: &Path, source: &str) -> Result<Option<ParsedFile>> {
        let Some(language) = Language::from_path(path) else {
            return Ok(None);
        };
        if !language.has_grammar() {
            return Ok(Some(ParsedFile {
                language,
                hits: Vec::new(),
                parsed: false,
            }));
        }
        self.ensure(language)?;
        let tree = self
            .parsers
            .get_mut(&language)
            .expect("parser")
            .parse(source, None);
        let Some(tree) = tree else {
            return Ok(Some(ParsedFile {
                language,
                hits: Vec::new(),
                parsed: false,
            }));
        };
        let query = self.queries.get(&language).expect("query");
        let hits = collect_hits(query, &tree, source);
        Ok(Some(ParsedFile {
            language,
            hits,
            parsed: true,
        }))
    }

    fn ensure(&mut self, language: Language) -> Result<()> {
        if self.parsers.contains_key(&language) {
            return Ok(());
        }
        let ts_lang = ts_language(language);
        let mut parser = Parser::new();
        parser
            .set_language(&ts_lang)
            .with_context(|| format!("load {} grammar", language.name()))?;
        let query = Query::new(&ts_lang, query_source(language))
            .with_context(|| format!("compile {} query", language.name()))?;
        self.parsers.insert(language, parser);
        self.queries.insert(language, query);
        Ok(())
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn ts_language(language: Language) -> TsLanguage {
    match language {
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        _ => unreachable!("grammar requested for {}", language.name()),
    }
}

fn query_source(language: Language) -> &'static str {
    match language {
        Language::Python => include_str!("../../queries/python.scm"),
        Language::JavaScript => include_str!("../../queries/javascript.scm"),
        Language::TypeScript | Language::Tsx => include_str!("../../queries/typescript.scm"),
        Language::Go => include_str!("../../queries/go.scm"),
        Language::Rust => include_str!("../../queries/rust.scm"),
        _ => "",
    }
}

fn collect_hits(query: &Query, tree: &Tree, source: &str) -> Vec<AstHit> {
    let mut cursor = QueryCursor::new();
    let bytes = source.as_bytes();
    let mut matches = cursor.matches(query, tree.root_node(), bytes);
    let mut hits = Vec::new();
    while let Some(query_match) = matches.next() {
        let mut kind: Option<AstHitKind> = None;
        let mut line = 1usize;
        let mut snippet = String::new();
        let mut name = String::new();
        let mut detail = String::new();
        for capture in query_match.captures {
            let cap_name = query.capture_names()[capture.index as usize];
            let text = capture.node.utf8_text(bytes).unwrap_or("").to_string();
            match cap_name {
                "function" => {
                    kind = Some(AstHitKind::Function);
                    set_line(&mut line, &mut snippet, source, capture.node);
                }
                "function.name" => name = text,
                "function.params" => detail = param_count(capture.node).to_string(),
                "endpoint" => {
                    kind = Some(AstHitKind::Endpoint);
                    set_line(&mut line, &mut snippet, source, capture.node);
                }
                "endpoint.path" => name = unquote(&text),
                "endpoint.method" => detail = text,
                "shell" => {
                    kind = Some(AstHitKind::ShellCall);
                    set_line(&mut line, &mut snippet, source, capture.node);
                }
                "shell.callee" => name = text,
                "database" => {
                    kind = Some(AstHitKind::DatabaseCall);
                    set_line(&mut line, &mut snippet, source, capture.node);
                }
                "database.callee" => name = text,
                _ => {}
            }
        }
        let Some(kind) = kind else {
            continue;
        };
        if matches!(kind, AstHitKind::Function | AstHitKind::Endpoint) && name.is_empty() {
            continue;
        }
        hits.push(AstHit {
            kind,
            line,
            snippet,
            name,
            detail,
        });
    }
    hits
}

fn set_line(line: &mut usize, snippet: &mut String, source: &str, node: Node) {
    let row = node.start_position().row;
    *line = row + 1;
    *snippet = source.lines().nth(row).unwrap_or("").trim().to_string();
}

fn param_count(node: Node) -> usize {
    if node.kind() == "identifier" {
        return 1;
    }
    node.named_child_count()
}

fn unquote(text: &str) -> String {
    let trimmed = text.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 2 {
        let (open, close) = (bytes[0], bytes[bytes.len() - 1]);
        if (open == b'"' && close == b'"')
            || (open == b'\'' && close == b'\'')
            || (open == b'`' && close == b'`')
        {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    for prefix in ["r\"", "f\"", "b\"", "u\""] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            if let Some(inner) = rest.strip_suffix('"') {
                return inner.to_string();
            }
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hits(file: &str, source: &str) -> Vec<AstHit> {
        let mut registry = ParserRegistry::new();
        let parsed = registry
            .parse_file(Path::new(file), source)
            .unwrap()
            .unwrap_or_else(|| panic!("no grammar for {file}"));
        assert!(parsed.parsed, "parse failed for {file}");
        parsed.hits
    }

    fn kinds(file: &str, source: &str) -> Vec<(AstHitKind, String, String)> {
        hits(file, source)
            .into_iter()
            .map(|hit| (hit.kind, hit.name, hit.detail))
            .collect()
    }

    #[test]
    fn python_functions_endpoints_and_calls() {
        let source = r#"
import subprocess
def greet(name, title):
    return name
@app.get("/items")
def list_items():
    return []
path("/admin", admin)
subprocess.run(["ls"])
os.system("ls")
db.conn.Exec("SELECT 1")
"#;
        let found = kinds("sample.py", source);
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::Function && h.1 == "greet"));
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::Function && h.1 == "list_items"));
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::Endpoint && h.1 == "/items" && h.2 == "get"));
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::Endpoint && h.1 == "/admin" && h.2 == "path"));
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::ShellCall && h.1 == "subprocess.run"));
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::ShellCall && h.1 == "os.system"));
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::DatabaseCall && h.1 == "db.conn.Exec"));
        assert!(!found
            .iter()
            .any(|h| h.1.contains("subprocess") && h.0 == AstHitKind::DatabaseCall));
    }

    #[test]
    fn javascript_and_typescript_calls() {
        for file in ["sample.js", "sample.ts", "sample.tsx"] {
            let source = r#"
function greet(name) { return name; }
const arrow = (x, y) => x + y;
app.get("/items", handler);
execSync("ls");
exec("ls");
db.Exec("SELECT 1");
re.exec("a");
"#;
            let found = kinds(file, source);
            assert!(
                found
                    .iter()
                    .any(|h| h.0 == AstHitKind::Function && h.1 == "greet"),
                "{file}: {found:?}"
            );
            assert!(
                found
                    .iter()
                    .any(|h| h.0 == AstHitKind::Function && h.1 == "arrow"),
                "{file}: {found:?}"
            );
            assert!(
                found
                    .iter()
                    .any(|h| h.0 == AstHitKind::Endpoint && h.1 == "/items"),
                "{file}: {found:?}"
            );
            assert!(
                found
                    .iter()
                    .any(|h| h.0 == AstHitKind::ShellCall && h.1 == "execSync"),
                "{file}: {found:?}"
            );
            assert!(
                found
                    .iter()
                    .any(|h| h.0 == AstHitKind::ShellCall && h.1 == "exec"),
                "{file}: {found:?}"
            );
            assert!(
                found
                    .iter()
                    .any(|h| h.0 == AstHitKind::DatabaseCall && h.1 == "db.Exec"),
                "{file}: {found:?}"
            );
            assert!(
                !found.iter().any(|h| h.1 == "re.exec"),
                "{file} classified re.exec: {found:?}"
            );
        }
    }

    #[test]
    fn go_shell_is_not_database() {
        let source = r#"
package main
func Greet(name string) string { return name }
func (s *Store) Save(row string) error { return nil }
func routes() {
    http.HandleFunc("/items", listItems)
    router.GET("/items", createItem)
    exec.Command("az", "account", "show").Run()
    exec.CommandContext(ctx, "aws").Run()
    db.conn.Exec(m)
    tx.Exec("DELETE FROM t")
}
"#;
        let found = kinds("sample.go", source);
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::Function && h.1 == "Greet"));
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::Function && h.1 == "Save"));
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::Endpoint && h.1 == "/items"));
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::ShellCall && h.1 == "exec.Command"));
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::ShellCall && h.1 == "exec.CommandContext"));
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::DatabaseCall && h.1 == "db.conn.Exec"));
        assert!(found
            .iter()
            .any(|h| h.0 == AstHitKind::DatabaseCall && h.1 == "tx.Exec"));
    }

    #[test]
    fn rust_commands_and_routes() {
        let source = r#"
fn greet(name: &str) -> &str { name }
impl Store { fn save(&self, row: &str) {} }
fn routes() {
    app.route("/items", get(list_items));
    web::resource("/health");
    std::process::Command::new("az");
    Command::new("terraform");
    db.exec("SELECT 1");
}
"#;
        let found = kinds("sample.rs", source);
        assert!(
            found
                .iter()
                .any(|h| h.0 == AstHitKind::Function && h.1 == "greet"),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|h| h.0 == AstHitKind::Function && h.1 == "save"),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|h| h.0 == AstHitKind::Endpoint && h.1 == "/items"),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|h| h.0 == AstHitKind::Endpoint && h.1 == "/health"),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|h| h.0 == AstHitKind::ShellCall && h.1 == "std::process::Command::new"),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|h| h.0 == AstHitKind::ShellCall && h.1 == "Command::new"),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|h| h.0 == AstHitKind::DatabaseCall && h.1 == "db.exec"),
            "{found:?}"
        );
    }
}
