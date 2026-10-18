; Rust AST queries for Recon.
; Shell callees include std::process::Command::new, tokio::process::Command::new, and Command::new.
; Database callees use the same receiver and method names as the Go patterns.

(function_item
  name: (identifier) @function.name
  parameters: (parameters) @function.params) @function

(call_expression
  function: (field_expression
    field: (field_identifier) @endpoint.method)
  arguments: (arguments
    (string_literal) @endpoint.path)
  (#eq? @endpoint.method "route")) @endpoint

(call_expression
  function: (scoped_identifier
    name: (identifier) @endpoint.method)
  arguments: (arguments
    (string_literal) @endpoint.path)
  (#match? @endpoint.method "^(resource|route|get|post|put|delete|patch)$")) @endpoint

(call_expression
  function: (_) @shell.callee
  (#match? @shell.callee "^(subprocess(\\.[A-Za-z_][A-Za-z0-9_]*)?|os\\.system|os\\.popen|exec\\.Command(Context)?|syscall\\.Exec|child_process(\\.[A-Za-z_][A-Za-z0-9_]*)?|execFileSync|execFile|execSync|spawnSync|exec|shell_exec|passthru|code_execution|shell_tool|BashTool|PythonREPLTool|std::process::Command(::new)?|tokio::process::Command(::new)?|Command::new)$")) @shell

(call_expression
  function: (_) @database.callee
  (#match? @database.callee "^(statement|stmt|conn|rows|sql|db|tx)(\\.[A-Za-z_][A-Za-z0-9_]*)*\\.(Exec|exec|Query|query|QueryRow|queryRow|Prepare|prepare)$")) @database
