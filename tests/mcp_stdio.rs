//! The MCP server speaks stdio JSON-RPC and its tools stay inside the process.

use rmcp::model::CallToolRequestParams;
use rmcp::transport::TokioChildProcess;
use rmcp::ServiceExt;
use tokio::process::Command;

#[tokio::test]
async fn serve_lists_and_calls_tools() {
    let bin = env!("CARGO_BIN_EXE_tracelamp");
    let home = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(
        repo.path().join("main.go"),
        "package main\nimport \"os/exec\"\nfunc main() { exec.Command(\"az\").Run() }\n",
    )
    .unwrap();
    let token = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
    std::fs::create_dir(repo.path().join(".warp")).unwrap();
    std::fs::write(
        repo.path().join(".warp/.mcp.json"),
        format!(
            r#"{{"mcpServers":{{"remote":{{"url":"https://example.invalid/mcp","headers":{{"Authorization":"Bearer {token}"}}}}}}}}"#
        ),
    )
    .unwrap();
    let before = std::fs::read_dir(repo.path()).unwrap().count();

    let mut command = Command::new(bin);
    command
        .arg("mcp")
        .arg("serve")
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("TRACELAMP_SCAN_ROOT", repo.path());
    let transport = TokioChildProcess::new(command).unwrap();
    let mut client = ().serve(transport).await.expect("initialize tracelamp mcp server");

    let info = client.peer_info().expect("server info");
    let server = info.server_info.as_ref().expect("server implementation");
    assert_eq!(server.name, "tracelamp");

    let tools = client.list_all_tools().await.unwrap();
    let names: Vec<_> = tools.iter().map(|tool| tool.name.to_string()).collect();
    for expected in [
        "scan_repo",
        "list_findings",
        "explain_finding",
        "agent_inventory",
    ] {
        assert!(names.iter().any(|name| name == expected), "{names:?}");
    }

    let explained = call(
        client.peer(),
        "explain_finding",
        serde_json::json!({"rule_id": "recon/shell-execution"}),
    )
    .await;
    assert!(explained.contains("db.Exec"), "{explained}");

    let scanned = call(
        client.peer(),
        "scan_repo",
        serde_json::json!({"path": repo.path().to_str().unwrap()}),
    )
    .await;
    assert!(scanned.contains("recon/shell-execution"), "{scanned}");
    assert!(scanned.contains("Warp"), "{scanned}");
    assert!(!scanned.contains(token), "{scanned}");
    assert!(scanned.contains("mcp-header"), "{scanned}");

    let listed = call(
        client.peer(),
        "list_findings",
        serde_json::json!({"path": repo.path().to_str().unwrap(), "severity": "high"}),
    )
    .await;
    assert!(listed.contains("recon/shell-execution"), "{listed}");
    assert!(!listed.contains(token), "{listed}");

    let inventory = call(
        client.peer(),
        "agent_inventory",
        serde_json::json!({"path": repo.path().to_str().unwrap()}),
    )
    .await;
    assert!(inventory.contains("Warp"), "{inventory}");
    assert!(!inventory.contains(token), "{inventory}");

    let outside = tempfile::tempdir().unwrap();
    let denied = call_raw(
        client.peer(),
        "scan_repo",
        serde_json::json!({"path": outside.path().to_str().unwrap()}),
    )
    .await;
    assert_eq!(denied.is_error, Some(true), "{denied:?}");

    assert_eq!(std::fs::read_dir(repo.path()).unwrap().count(), before);
    client.close().await.unwrap();
}

async fn call(
    client: &rmcp::Peer<rmcp::RoleClient>,
    name: &str,
    arguments: serde_json::Value,
) -> String {
    let result = call_raw(client, name, arguments).await;
    assert_ne!(result.is_error, Some(true), "{result:?}");
    result
        .content
        .iter()
        .filter_map(|block| block.as_text().map(|text| text.text.as_str()))
        .collect()
}

async fn call_raw(
    client: &rmcp::Peer<rmcp::RoleClient>,
    name: &str,
    arguments: serde_json::Value,
) -> rmcp::model::CallToolResult {
    let object = arguments.as_object().unwrap().clone();
    client
        .call_tool(CallToolRequestParams::new(name.to_string()).with_arguments(object))
        .await
        .unwrap_or_else(|err| panic!("{name} failed: {err}"))
}
