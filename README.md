# Recon CLI

A Rust CLI tool that scans repositories for API endpoints, functions, database connections, and agents analyzing security risks, API resource efficiency, and agent-data interactions. Extensible via skills, agents, and MCP servers, with support for both open-source and paid LLM backends.

## Language Choice: Rust
* **Why Rust over Go/C:** Official MCP SDK (`rmcp`), mature tree-sitter bindings for multi-language AST parsing, memory safety without GC, strong async ecosystem (tokio), and rich crate ecosystem for LLM clients. Go lacks official MCP SDK maturity; C is too low-level for this scope.
* 
## Core Dependencies
* `clap` CLI framework with subcommands
* `tree-sitter` (v0.26+) multi-language AST parsing (grammars: `tree-sitter-python`, `tree-sitter-javascript`, `tree-sitter-typescript`, `tree-sitter-rust`, `tree-sitter-go`, `tree-sitter-java`, etc.)
* `rmcp` (v0.16+) official Rust MCP SDK (client & server, stdio + HTTP/SSE transports)
* `llm-connector` unified LLM client (OpenAI, Anthropic, Aliyun/Qwen, Ollama)
* `tokio` async runtime
* `serde` / `serde_json` serialization
* `reqwest` HTTP client for API calls
* `regex` pattern-based scanning (secrets, connection strings)
* `walkdir` recursive directory traversal

## Project Structure

```warp-runnable-command
recon/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point (clap)
│   ├── config.rs            # Config loading (TOML/YAML)
│   ├── scanner/
│   │   ├── mod.rs            # Scanner orchestrator
│   │   ├── api_endpoints.rs  # Detect REST/GraphQL/gRPC endpoints
│   │   ├── functions.rs      # Function extraction & complexity analysis
│   │   ├── db_connections.rs # DB connection string & query detection
│   │   ├── agents.rs         # Agent config detection (LangChain, AutoGen, MCP, etc.)
│   │   └── secrets.rs        # Exposed credentials & secrets scanning
│   ├── analysis/
│   │   ├── mod.rs            # Analysis orchestrator
│   │   ├── security.rs       # Security risk scoring & vulnerability patterns
│   │   ├── api_efficiency.rs # Rate limits, N+1 queries, redundant calls
│   │   └── agent_data.rs     # Agent permission scope, data flow analysis
│   ├── parsers/
│   │   ├── mod.rs            # Tree-sitter parser registry
│   │   └── queries/          # Tree-sitter query files per language (.scm)
│   ├── llm/
│   │   ├── mod.rs            # LLM provider abstraction trait
│   │   ├── providers.rs      # Anthropic, OpenAI, Ollama, Mistral, Qwen
│   │   └── prompts.rs        # Analysis prompt templates
│   ├── mcp/
│   │   ├── mod.rs            # MCP client + server setup
│   │   ├── client.rs         # Connect to external MCP servers
│   │   └── server.rs         # Expose scanner as MCP server (tools)
│   ├── skills/
│   │   ├── mod.rs            # Skill loader & registry
│   │   └── skill.rs          # Skill trait & dynamic loading
│   ├── agents/
│   │   ├── mod.rs            # Agent orchestration
│   │   └── agent.rs          # Agent trait (scan, analyze, report)
│   └── report/
│       ├── mod.rs            # Report generation
│       ├── json.rs           # JSON output
│       ├── sarif.rs          # SARIF format (CI/CD integration)
│       └── terminal.rs       # Rich terminal output
├── skills/                   # Built-in skill definitions (TOML/YAML)
├── config/
│   └── default.toml          # Default configuration
└── tests/
```

## Implementation Phases

### Phase 1: Foundation (CLI + Scanning Engine)
1. Initialize Cargo workspace, add core deps (`clap`, `tree-sitter`, `walkdir`, `serde`, `tokio`)
2. Build CLI with subcommands: `scan`, `analyze`, `report`, `skill`, `agent`, `mcp`
3. Implement tree-sitter parser registry load grammars dynamically per file extension
4. Build scanners using tree-sitter queries (.scm):
    * **API endpoints:** route decorators (`@app.get`, `router.Handle`), HTTP method annotations, OpenAPI/Swagger refs
    * **Functions:** extract signatures, params, return types, cyclomatic complexity
    * **DB connections:** connection string patterns (postgres://, mongodb://, mysql://), ORM model definitions, raw SQL queries
    * **Secrets:** regex patterns for API keys, tokens, passwords in source (AWS, GCP, GitHub, Stripe, etc.)
5. Config file support (TOML) for scan rules, ignore paths, severity thresholds
   
### Phase 2: AI-Powered Analysis

1. Implement `LlmProvider` trait with methods: `analyze()`, `summarize()`, `suggest_fix()`
2. Build provider implementations:
    * **Ollama** (local open-source: Qwen, Mistral, Llama, DeepSeek) via OpenAI-compatible API at localhost:11434
    * **Anthropic** (Claude) via `llm-connector` or direct `reqwest` calls to `api.anthropic.com`
    * **OpenAI** (GPT-4) via OpenAI protocol
    * **Mistral API** via OpenAI-compatible endpoint
3. Prompt engineering for each analysis type:
    * Security: "Given this code context, identify OWASP Top 10 vulnerabilities..."
    * API efficiency: "Analyze these API calls for N+1 patterns, missing pagination..."
    * Agent data: "Evaluate this agent's data access scope and potential exfiltration paths..."
4. Streaming response support for real-time terminal feedback
   
### Phase 3: MCP Integration

1. **MCP Client** (using `rmcp`): connect to external MCP servers for extended scanning capabilities
    * Discover & invoke tools from registered MCP servers
    * Use stdio and HTTP/SSE transports
2. **MCP Server** (using `rmcp`): expose scanner as an MCP server so AI agents/IDEs can invoke it
    * Tools: `scan_repo`, `analyze_file`, `get_report`, `list_findings`
    * Resources: expose scan results as MCP resources
3. MCP server config in `config/default.toml` — list of MCP servers to connect to
   
### Phase 4: Skills & Agent System

1. **Skills:** Pluggable analysis modules loaded from TOML/YAML definitions
    * Each skill defines: name, description, target patterns, tree-sitter queries, LLM prompts
    * CLI: `recon skill add <path>`, `recon skill list`, `recon skill remove <name>`
    * Skills directory scanned at startup; hot-reload on config change
2. **Agents:** Autonomous analysis workflows that compose scanners + LLM + skills
    * Agent trait: `scan()` → `analyze()` → `report()`
    * Built-in agents: SecurityAuditor, ApiOptimizer, AgentInspector
    * CLI: `recon agent add <config>`, `recon agent run <name> --target <path>`
    * Agents can invoke MCP tools and chain multiple skills
  
### Phase 5: Reporting & Distribution
1. Output formats: JSON, SARIF (for GitHub/GitLab CI), rich terminal (colored, tables)
2. Severity scoring: Critical / High / Medium / Low / Info
3. Cross-platform binaries (cargo build targets: Windows, Linux, macOS)
4. Cross-platform binaries via `cargo-dist` or GitHub Releases
   
## CLI Interface
```warp-runnable-command
recon scan <path>               # Scan a repo
recon scan <path> --format json  # JSON output
recon analyze <path>             # AI-powered deep analysis
recon analyze <path> --provider ollama --model qwen3-coder
recon report <path> --format sarif
recon skill add ./my-skill.toml
recon skill list
recon agent run security-auditor --target ./my-repo
recon mcp serve                  # Start as MCP server
recon mcp connect <uri>          # Connect to MCP server
recon config set llm.provider anthropic
recon config set llm.api_key {{ANTHROPIC_API_KEY}}
```
