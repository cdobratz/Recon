use regex::Regex;
use std::sync::LazyLock;

/// Whether a source line is a shell/process spawn or a database call.
///
/// `db.Exec`, `tx.Exec`, and `stmt.Exec` are database calls. They must not be
/// reported as shell execution. `exec.Command`, `subprocess`, `os.system`, and
/// `child_process` are real process spawns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionClass {
    Shell,
    Database,
    None,
}

pub fn classify_execution(line: &str) -> ExecutionClass {
    if is_database_call(line) {
        return ExecutionClass::Database;
    }
    if is_shell_call(line) {
        return ExecutionClass::Shell;
    }
    ExecutionClass::None
}

fn is_database_call(line: &str) -> bool {
    static DB_CALL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\b(?:db|tx|stmt|conn|sql|statement|rows)(?:\s*\.\s*[A-Za-z_]\w*)*\s*\.\s*(?:Exec|Query|QueryRow|Prepare)\s*\(",
        )
        .unwrap()
    });
    DB_CALL.is_match(line)
}

fn is_shell_call(line: &str) -> bool {
    static SHELL_CALL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"\bexec\.Command(?:Context)?\s*\(|\bsyscall\.Exec\s*\(|\bsubprocess\b|\bos\.system\s*\(|\bos\.popen\s*\(|\bchild_process\b|\b(?:execFileSync|execFile|execSync|spawnSync)\s*\(|(?:^|[^.\w])exec\s*\(|\bshell_exec\s*\(|\bpassthru\s*\(|\bcode_execution\b|\bshell_tool\b|\bBashTool\b|\bPythonREPLTool\b|\bstd::process::Command\b|\btokio::process::Command\b|\bCommand::new\s*\(",
        )
        .unwrap()
    });
    SHELL_CALL.is_match(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_exec_is_not_shell() {
        for line in [
            "if _, err := db.conn.Exec(m); err != nil {",
            "_, err := db.conn.Exec(`INSERT INTO config`)",
            "_, err := stmt.Exec(r.SubscriptionID, r.ResourceGroup)",
            "_, err := tx.Exec(`UPDATE users SET status = ?`)",
            "db.Exec(\"SELECT 1\")",
            "rows, err := db.conn.Query(query, args...)",
        ] {
            assert_eq!(classify_execution(line), ExecutionClass::Database, "{line}");
        }
    }

    #[test]
    fn real_shell_calls_are_shell() {
        for line in [
            "cmd := exec.Command(\"az\", \"account\", \"show\")",
            "cmd := exec.CommandContext(ctx, \"aws\", \"configure\", \"export-credentials\")",
            "import subprocess",
            "subprocess.run([\"ls\"])",
            "os.system(\"ls\")",
            "const { exec } = require(\"child_process\");",
            "exec(\"ls\")",
            "execSync(\"ls\")",
            "std::process::Command::new(\"az\")",
            "Command::new(\"terraform\")",
        ] {
            assert_eq!(classify_execution(line), ExecutionClass::Shell, "{line}");
        }
    }

    #[test]
    fn regex_exec_is_not_shell() {
        assert_eq!(
            classify_execution("const m = re.exec(text);"),
            ExecutionClass::None
        );
        assert_eq!(
            classify_execution("return t.execute(wr, data)"),
            ExecutionClass::None
        );
    }
}
