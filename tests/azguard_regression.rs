//! Regression against cdobratz/AzGuard: database Exec is not shell, real
//! exec.Command calls are, and high-severity findings are not false positives.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const AZGUARD_REV: &str = "bdeb046116f29541185f884ce6c940d44299cec8";

#[test]
fn azguard_has_no_high_severity_false_positives() {
    let root = ensure_azguard();
    let output = Command::new(env!("CARGO_BIN_EXE_recon"))
        .args(["scan", root.to_str().unwrap(), "--format", "json"])
        .output()
        .expect("run recon");
    assert!(
        output.status.success(),
        "scan failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|err| panic!("json ({err}): {}", String::from_utf8_lossy(&output.stdout)));
    let findings = value["findings"].as_array().expect("findings");
    let highs: Vec<&Value> = findings
        .iter()
        .filter(|finding| finding["severity"] == "high")
        .collect();

    let mut failures = Vec::new();
    for finding in &highs {
        let snippet = finding["snippet"].as_str().unwrap_or("");
        let rule = finding["metadata"]["rule_id"].as_str().unwrap_or("");
        let file = finding["file"].as_str().unwrap_or("");
        if snippet.contains(".Exec(")
            || snippet.contains(".Query(")
            || snippet.contains(".QueryRow(")
        {
            failures.push(format!("database call flagged high: {file}: {snippet}"));
        }
        if rule != "recon/shell-execution" {
            failures.push(format!("unexpected high rule {rule} at {file}: {snippet}"));
        } else if !snippet.contains("exec.Command") {
            failures.push(format!(
                "shell rule is not a real exec.Command: {file}: {snippet}"
            ));
        }
        if file.contains("/.go/") {
            failures.push(format!("vendored toolchain was scanned: {file}"));
        }
    }

    let expected = count_exec_command_lines(&root);
    if highs.len() != expected {
        failures.push(format!(
            "high findings {} != exec.Command lines {expected}",
            highs.len()
        ));
    }
    assert!(
        highs.iter().any(|finding| finding["file"]
            .as_str()
            .unwrap_or("")
            .ends_with("internal/cloud/azure/auth.go")),
        "missed exec.Command in auth.go; highs={}",
        highs.len()
    );
    assert!(
        !highs.iter().any(|finding| finding["file"]
            .as_str()
            .unwrap_or("")
            .ends_with("internal/storage/sqlite.go")),
        "sqlite.go database calls were flagged high"
    );
    assert!(
        failures.is_empty(),
        "AzGuard high-severity false positives:\n{}",
        failures.join("\n")
    );
    println!(
        "azguard regression highs={} exec.Command lines={} false_positives=0",
        highs.len(),
        expected
    );
}

fn count_exec_command_lines(root: &Path) -> usize {
    let mut count = 0;
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
        if ignored(rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        count += text
            .lines()
            .filter(|line| line.contains("exec.Command"))
            .count();
    }
    count
}

fn ignored(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some(".go" | ".git" | "vendor" | "dist" | "node_modules" | "target")
        )
    })
}

fn ensure_azguard() -> PathBuf {
    let dest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/azguard-fixture");
    if dest.join("internal/storage/sqlite.go").is_file() && rev_is_pinned(&dest) {
        return dest;
    }
    if dest.exists() {
        std::fs::remove_dir_all(&dest).expect("remove stale fixture");
    }
    let status = Command::new("git")
        .args([
            "clone",
            "--filter=blob:none",
            "--sparse",
            "--depth",
            "1",
            "https://github.com/cdobratz/AzGuard.git",
        ])
        .arg(&dest)
        .status()
        .expect("git clone");
    assert!(status.success(), "git clone AzGuard failed");
    git(&dest, &["fetch", "--depth", "1", "origin", AZGUARD_REV]);
    git(&dest, &["checkout", "--detach", "FETCH_HEAD"]);
    git(
        &dest,
        &[
            "sparse-checkout",
            "set",
            "--no-cone",
            "/cmd/**",
            "/internal/**",
            "/configs/**",
            "/go.mod",
            "/README.md",
        ],
    );
    assert!(
        rev_is_pinned(&dest),
        "AzGuard checkout is not {AZGUARD_REV}"
    );
    assert!(
        dest.join("internal/storage/sqlite.go").is_file(),
        "sparse checkout is missing sqlite.go"
    );
    dest
}

fn rev_is_pinned(dest: &Path) -> bool {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dest)
        .output();
    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim() == AZGUARD_REV
        }
        _ => false,
    }
}

fn git(dest: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dest)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
