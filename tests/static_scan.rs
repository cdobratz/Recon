//! `recon scan` must not spawn processes or open network connections.

use std::process::Command;

#[test]
fn scan_does_not_spawn_or_connect() {
    let bin = env!("CARGO_BIN_EXE_recon");
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("main.go"),
        "package main\nimport \"os/exec\"\nfunc main() { exec.Command(\"az\", \"account\", \"show\").Run() }\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join(".cursor")).unwrap();
    std::fs::write(
        dir.path().join(".cursor/mcp.json"),
        r#"{"mcpServers":{"remote":{"command":"npx","args":["-y","mcp-remote@0.1.16","https://example.invalid/mcp"]}}}"#,
    )
    .unwrap();

    let trace = dir.path().join("trace.txt");
    let output = Command::new("strace")
        .args([
            "-f",
            "-e",
            "trace=execve,execveat,connect,sendto,sendmsg,sendmmsg",
            "-s",
            "200",
            "-o",
        ])
        .arg(&trace)
        .arg(bin)
        .arg("scan")
        .arg(dir.path())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap_or_else(|err| panic!("strace is required for the static-scan check: {err}"));
    assert!(
        output.status.success(),
        "scan failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let text = std::fs::read_to_string(&trace).expect("read trace");
    let mut execs = Vec::new();
    let mut network = Vec::new();
    for line in text.lines() {
        if is_syscall(line, "execve") || is_syscall(line, "execveat") {
            execs.push(line.to_string());
            continue;
        }
        if is_syscall(line, "connect")
            || is_syscall(line, "sendto")
            || is_syscall(line, "sendmsg")
            || is_syscall(line, "sendmmsg")
        {
            if line.contains("AF_UNIX") || line.contains("AF_LOCAL") {
                continue;
            }
            network.push(line.to_string());
        }
    }

    assert!(
        network.is_empty(),
        "scan opened a network connection:\n{}",
        network.join("\n")
    );
    for exec in &execs {
        let allowed = exec.contains(bin) || exec.contains("/recon");
        assert!(
            allowed,
            "scan spawned a process:\n{exec}\nfull trace:\n{text}"
        );
    }
    assert!(
        execs.len() <= 1,
        "scan spawned a process:\n{}\nfull trace:\n{text}",
        execs.join("\n")
    );
}

fn is_syscall(line: &str, name: &str) -> bool {
    let trimmed = line.trim_start();
    let rest = trimmed
        .split_once(' ')
        .map(|(pid, rest)| {
            if pid.chars().all(|c| c.is_ascii_digit()) {
                rest
            } else {
                trimmed
            }
        })
        .unwrap_or(trimmed);
    rest.starts_with(name) && rest[name.len()..].starts_with('(')
}
