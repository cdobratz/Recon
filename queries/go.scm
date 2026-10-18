; Go AST queries for Recon.
; Shell callees include exec.Command and exec.CommandContext.
; Database callees include db.Exec, db.conn.Exec, tx.Exec, and stmt.Exec.

(function_declaration
  name: (identifier) @function.name
  parameters: (parameter_list) @function.params) @function

(method_declaration
  name: (field_identifier) @function.name
  parameters: (parameter_list) @function.params) @function

(call_expression
  function: (selector_expression
    field: (field_identifier) @endpoint.method)
  arguments: (argument_list
    [(interpreted_string_literal) (raw_string_literal)] @endpoint.path)
  (#match? @endpoint.method "^(HandleFunc|Handle|GET|POST|PUT|DELETE|PATCH)$")) @endpoint

(call_expression
  function: (_) @shell.callee
  (#match? @shell.callee "^(subprocess(\\.[A-Za-z_][A-Za-z0-9_]*)?|os\\.system|os\\.popen|exec\\.Command(Context)?|syscall\\.Exec|child_process(\\.[A-Za-z_][A-Za-z0-9_]*)?|execFileSync|execFile|execSync|spawnSync|exec|shell_exec|passthru|code_execution|shell_tool|BashTool|PythonREPLTool|std::process::Command(::new)?|tokio::process::Command(::new)?|Command::new)$")) @shell

(call_expression
  function: (_) @database.callee
  (#match? @database.callee "^(statement|stmt|conn|rows|sql|db|tx)(\\.[A-Za-z_][A-Za-z0-9_]*)*\\.(Exec|exec|Query|query|QueryRow|queryRow|Prepare|prepare)$")) @database
