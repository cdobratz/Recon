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

    let mut command = Command::new(bin);
    command
        .arg("mcp")
        .arg("serve")
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME");
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

    client.close().await.unwrap();
}

async fn call(
    client: &rmcp::Peer<rmcp::RoleClient>,
    name: &str,
    arguments: serde_json::Value,
) -> String {
    let object = arguments.as_object().unwrap().clone();
    let result = client
        .call_tool(CallToolRequestParams::new(name.to_string()).with_arguments(object))
        .await
        .unwrap_or_else(|err| panic!("{name} failed: {err}"));
    assert_ne!(result.is_error, Some(true), "{result:?}");
    result
        .content
        .iter()
        .filter_map(|block| block.as_text().map(|text| text.text.as_str()))
        .collect()
}
