//! The `recon` binary stays installed and runs the same scanner as `tracelamp`.

use std::process::Command;

#[test]
fn recon_command_alias_scans() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.go"), "package main\nfunc main() {}\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_recon"))
        .args(["scan", dir.path().to_str().unwrap(), "--format", "json"])
        .output()
        .expect("run recon alias");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("files_scanned"), "{stdout}");
}

#[test]
fn help_uses_the_tracelamp_command_name() {
    let output = Command::new(env!("CARGO_BIN_EXE_tracelamp"))
        .arg("--help")
        .output()
        .expect("tracelamp --help");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("tracelamp"), "{text}");
    assert!(text.contains("TraceLamp"), "{text}");
}
