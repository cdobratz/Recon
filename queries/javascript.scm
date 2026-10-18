; JavaScript AST queries for Recon.
; Captures: function, endpoint, shell call, database call.
; Shell callees: child_process, exec/execSync/execFile/spawnSync, and the shared process-spawn forms.
; Database callees: db/tx/stmt/conn/sql/statement/rows . Exec/Query/QueryRow/Prepare.

(function_declaration
  name: (identifier) @function.name
  parameters: (formal_parameters) @function.params) @function

(function_expression
  name: (identifier) @function.name
  parameters: (formal_parameters) @function.params) @function

(generator_function
  name: (identifier) @function.name
  parameters: (formal_parameters) @function.params) @function

(generator_function_declaration
  name: (identifier) @function.name
  parameters: (formal_parameters) @function.params) @function

(lexical_declaration
  (variable_declarator
    name: (identifier) @function.name
    value: (arrow_function
      parameters: (_) @function.params))) @function

(lexical_declaration
  (variable_declarator
    name: (identifier) @function.name
    value: (function_expression
      parameters: (formal_parameters) @function.params))) @function

(method_definition
  name: (property_identifier) @function.name
  parameters: (formal_parameters) @function.params) @function

(call_expression
  function: (member_expression
    object: (identifier) @endpoint.object
    property: (property_identifier) @endpoint.method)
  arguments: (arguments
    [(string) (template_string)] @endpoint.path)
  (#match? @endpoint.object "^(app|router)$")
  (#match? @endpoint.method "^(get|post|put|delete|patch|all)$")) @endpoint

(call_expression
  function: (_) @shell.callee
  (#match? @shell.callee "^(subprocess(\\.[A-Za-z_][A-Za-z0-9_]*)?|os\\.system|os\\.popen|exec\\.Command(Context)?|syscall\\.Exec|child_process(\\.[A-Za-z_][A-Za-z0-9_]*)?|execFileSync|execFile|execSync|spawnSync|exec|shell_exec|passthru|code_execution|shell_tool|BashTool|PythonREPLTool|std::process::Command(::new)?|tokio::process::Command(::new)?|Command::new)$")) @shell

(call_expression
  function: (_) @database.callee
  (#match? @database.callee "^(statement|stmt|conn|rows|sql|db|tx)(\\.[A-Za-z_][A-Za-z0-9_]*)*\\.(Exec|exec|Query|query|QueryRow|queryRow|Prepare|prepare)$")) @database
