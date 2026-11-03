# Recon

Static, no-execute scanner for what AI agents and MCP servers can touch: tools, data stores, credentials, and cloud spend. A scan is **static, no-execute, and CI-safe**. It never spawns processes and never opens network connections, so it can run on untrusted pull requests without a sandbox.

Recon does not start MCP servers to read their tool lists. Config files are parsed as text.

## Implemented / Planned

| Area | Status |
|---|---|
| `recon scan` for API endpoints, functions, DB connections, agents, and secrets | Implemented |
| Terminal, JSON, and SARIF 2.1.0 output | Implemented |
| Shell vs database call precision (`exec.Command` is shell, `db.Exec` is not) | Implemented |
| Static, no-execute, CI-safe scan | Implemented |
| Agent/MCP config inventory (JSON + Mermaid) | Implemented |
| MCP rules (unpinned npx/uvx, mcp-remote CVE-2025-6514, secrets in env, filesystem `/` or `~`, shell without approval) | Implemented |
| "Agent can spend cloud money" rule, with remediation pointing at AzGuard | Implemented |
| Tree-sitter queries for Python, JavaScript, TypeScript, Go, and Rust (functions, API endpoints, shell vs database calls) | Implemented |
| Regex fallback for other languages | Implemented |
| Default ignores, `.gitignore`, and `.reconignore` | Implemented |
| Inline `recon:ignore` suppressions and a baseline file | Implemented |
| CycloneDX 1.6 AI-BOM export (`--format cyclonedx`) | Implemented |
| GitHub Actions CI (`cargo fmt --check`, build, test, clippy) | Implemented |
| Opt-in LLM analysis (`recon analyze --llm`, local Ollama by default, hosted OpenAI and Anthropic) | Implemented |
| MCP server (`recon mcp serve`, stdio and streamable HTTP) with `scan_repo`, `list_findings`, `explain_finding`, `agent_inventory` | Implemented |
| MCP client (`recon mcp connect` for a stdio command or an http(s) URL) | Implemented |
| Skills (`recon skill run`) and agent workflows (`recon agent run`) | Implemented |

## Static scan guarantee

`recon scan` and `recon report` only read files under the path you pass. They do not:

- spawn `npx`, `uvx`, shells, or any other process
- connect to MCP server URLs, package registries, or LLM APIs

The check is `tests/static_scan.rs`. It runs `scan` and `analyze` (without `--llm`) under `strace` and fails if the process calls `execve` (other than the scanner itself) or a network syscall.

`recon analyze --llm` is opt-in. It sends findings and short snippets to the model, never whole files, and stops when the estimated cost would pass `llm.max_cost_usd` (0.25 by default). The default provider is local Ollama (`llama3.2` at `http://localhost:11434`, or `OLLAMA_HOST`). `--provider openai` uses `OPENAI_API_KEY` and `--provider anthropic` uses `ANTHROPIC_API_KEY`. Ollama is priced at zero, so it always fits the cap.

`recon mcp serve` speaks MCP over stdio. `recon mcp serve --transport http --port 3000` listens on `0.0.0.0:3000` and serves the same tools at `/mcp` (streamable HTTP). `recon mcp connect` either spawns a stdio command or opens an `http://` / `https://` endpoint. Neither path is used by `scan`.

```bash
recon scan <path>
recon scan <path> --format json
recon scan <path> --format sarif
recon scan <path> --format mermaid
recon scan <path> --format cyclonedx
recon scan <path> --write-baseline recon-baseline.json
recon scan <path> --baseline recon-baseline.json
recon report <path> --format sarif
recon analyze <path>
recon analyze <path> --llm
recon analyze <path> --llm --provider openai --max-cost 0.25
recon mcp serve
recon mcp serve --transport http --port 3000
recon mcp connect "recon mcp serve"
recon mcp connect http://127.0.0.1:3000/mcp
```

JSON output includes `inventory` (agent → MCP server/tool → data store → credential) and `inventory_mermaid`. `--format mermaid` prints only the diagram. `--format cyclonedx` prints a CycloneDX 1.6 JSON document of that inventory (agents, MCP servers, tools, data stores, credentials).

`--baseline` prints only findings whose fingerprint is not already in the file and exits 1 when any remain, so CI fails on new findings. `--write-baseline` records the current findings and does not fail the run. A fingerprint is the SHA-256 of the rule id, the path relative to the scan root, and the snippet with whitespace collapsed. Line numbers are not part of it.

Suppress a finding by putting `recon:ignore <rule-id>` on the same line or the previous line. `//`, `#`, and `/* */` comments all work. Several rule ids can be separated by spaces or commas.

## Rules

Each rule has a stable id, help text, an [OWASP Top 10 for Agentic Applications 2026](https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/) ASI mapping, and a SARIF `tool.driver.rules[]` entry.

| Rule ID | ASI | What it flags |
|---|---|---|
| `recon/shell-execution` | ASI05 | Real process spawns (`exec.Command`, `subprocess`, `os.system`, `child_process`). Not `db.Exec`. |
| `recon/mcp-unpinned-package` | ASI04 | `npx` / `uvx` package with no exact version. `pkg@1.2.3` and `pkg==1.2.3` do not match. |
| `recon/mcp-remote-cve-2025-6514` | ASI05 | `mcp-remote` versions 0.0.5 through 0.1.15 (CVE-2025-6514). |
| `recon/mcp-secret-in-env` | ASI03 | Literal secret in an MCP `env` value. `${VAR}` references do not match. |
| `recon/mcp-filesystem-root` | ASI02 | Filesystem root set to `/` or `~`. |
| `recon/mcp-shell-without-approval` | ASI02, ASI05 | Shell or exec tool with no approval flag. |
| `recon/agent-cloud-spend` | ASI02 | Agent or MCP server with cloud credentials, a cloud CLI / IaC apply, or a cloud write tool. Remediation: least privilege and [`azguard guard`](https://github.com/cdobratz/AzGuard#agent-guard). |

Configs read for the inventory: `mcp.json`, `.mcp.json`, `.cursor/mcp.json`, `.vscode/mcp.json`, `claude_desktop_config.json`, and LangChain tool lists.

## Build

HTTP uses `reqwest` with `rustls-tls` (no OpenSSL / `libssl-dev`).

```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
```

Default ignores include `.git`, `node_modules`, `target`, `vendor`, `dist`, `build`, `__pycache__`, `.venv`, and `.go` (a vendored Go toolchain). The walk also honors `.gitignore` and `.reconignore` (gitignore syntax) and skips binary files (known extensions or a NUL byte) and files larger than `max_file_size` (1 MiB by default). Hidden paths such as `.cursor/mcp.json` are still scanned.

## Project structure

```
recon/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── scanner/          # in-process scanners, MCP inventory, rules
│   ├── analysis/         # rule-based scores; optional LLM pass
│   ├── parsers/          # tree-sitter parsers; regex fallback for other languages
│   ├── llm/              # Ollama, OpenAI, and Anthropic (only with --llm)
│   ├── mcp/              # stdio and streamable HTTP MCP server and client
│   ├── skills/
│   ├── agents/
│   └── report/           # terminal, JSON, SARIF, Mermaid
├── queries/              # tree-sitter .scm queries, one file per language
├── config/default.toml
└── tests/fixtures/       # rule fixtures, language corpus, and expected results
```

## CLI

```
recon scan <path>
recon scan <path> --format json
recon scan <path> --format cyclonedx
recon scan <path> --write-baseline recon-baseline.json
recon scan <path> --baseline recon-baseline.json
recon scan <path> --only agents,mcp
recon analyze <path>
recon analyze <path> --llm
recon report <path> --format sarif
recon skill add ./todo.toml
recon skill run todo --target <path>
recon agent run reviewer --target <path>
recon agent run reviewer --target <path> --llm
recon mcp serve
recon mcp serve --transport http --port 3000
recon mcp connect "recon mcp serve"
recon mcp connect http://127.0.0.1:3000/mcp
recon config show
```

`analyze` scores the static findings. Add `--llm` to also ask a model about those findings. A skill is a TOML file of regex search patterns; `skill run` and `agent run` apply them while reading files. An agent runs the static scan, then its skills, then the rule scores. `--llm` or `use_llm = true` in the agent file is what turns the model on.
