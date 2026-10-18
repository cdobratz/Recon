; Python AST queries for Recon.
; Captures: function, endpoint, shell call, database call.
; Shell callees: subprocess, os.system, os.popen, and the shared process-spawn forms.
; Database callees: db/tx/stmt/conn/sql/statement/rows . Exec/Query/QueryRow/Prepare.
; Regex remains the fallback when this grammar does not parse the file.

(function_definition
  name: (identifier) @function.name
  parameters: (parameters) @function.params) @function

(decorator
  (call
    function: (attribute
      object: (identifier) @endpoint.object
      attribute: (identifier) @endpoint.method)
    arguments: (argument_list
      (string) @endpoint.path))
  (#match? @endpoint.object "^(app|router|api)$")
  (#match? @endpoint.method "^(get|post|put|delete|patch|options|head|route)$")) @endpoint

(call
  function: (identifier) @endpoint.method
  arguments: (argument_list
    (string) @endpoint.path)
  (#eq? @endpoint.method "path")) @endpoint

(call
  function: (_) @shell.callee
  (#match? @shell.callee "^(subprocess(\\.[A-Za-z_][A-Za-z0-9_]*)?|os\\.system|os\\.popen|exec\\.Command(Context)?|syscall\\.Exec|child_process(\\.[A-Za-z_][A-Za-z0-9_]*)?|execFileSync|execFile|execSync|spawnSync|exec|shell_exec|passthru|code_execution|shell_tool|BashTool|PythonREPLTool|std::process::Command(::new)?|tokio::process::Command(::new)?|Command::new)$")) @shell

(call
  function: (_) @database.callee
  (#match? @database.callee "^(statement|stmt|conn|rows|sql|db|tx)(\\.[A-Za-z_][A-Za-z0-9_]*)*\\.(Exec|exec|Query|query|QueryRow|queryRow|Prepare|prepare)$")) @database
