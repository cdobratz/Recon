//! CycloneDX 1.6 JSON export of the agent and MCP inventory.
//!
//! This is a static AI-BOM style document. It does not fetch a schema and it
//! does not contact a registry.

use crate::scanner::mcp_inventory::{InventoryGraph, InventoryServer};
use crate::scanner::ScanResults;
use anyhow::Result;
use serde_json::{json, Value};

pub fn format_scan(results: &ScanResults) -> Result<String> {
    Ok(serde_json::to_string_pretty(&document(&results.inventory))?)
}

fn document(graph: &InventoryGraph) -> Value {
    let mut components = Vec::new();
    let mut dependencies = Vec::new();
    let mut agent_refs = Vec::new();

    for (ai, agent) in graph.agents.iter().enumerate() {
        let agent_ref = format!("agent-{ai}");
        agent_refs.push(agent_ref.clone());
        components.push(json!({
            "type": "application",
            "bom-ref": agent_ref,
            "name": agent.name,
            "properties": [
                { "name": "tracelamp:kind", "value": "agent" },
                { "name": "tracelamp:source", "value": agent.source }
            ]
        }));
        let mut server_refs = Vec::new();
        for (si, server) in agent.servers.iter().enumerate() {
            let server_ref = format!("server-{ai}-{si}");
            server_refs.push(server_ref.clone());
            components.push(server_component(&server_ref, server));
            let mut child_refs = Vec::new();
            for (ti, tool) in server.tools.iter().enumerate() {
                let tool_ref = format!("tool-{ai}-{si}-{ti}");
                child_refs.push(tool_ref.clone());
                components.push(json!({
                    "type": "library",
                    "bom-ref": tool_ref,
                    "name": tool.name,
                    "description": tool.description,
                    "properties": [{ "name": "tracelamp:kind", "value": "tool" }]
                }));
            }
            for (di, store) in server.data_stores.iter().enumerate() {
                let store_ref = format!("store-{ai}-{si}-{di}");
                child_refs.push(store_ref.clone());
                components.push(json!({
                    "type": "data",
                    "bom-ref": store_ref,
                    "name": store.name,
                    "properties": [
                        { "name": "tracelamp:kind", "value": "data-store" },
                        { "name": "tracelamp:store-kind", "value": store.kind }
                    ]
                }));
            }
            for (ci, cred) in server.credentials.iter().enumerate() {
                let cred_ref = format!("cred-{ai}-{si}-{ci}");
                child_refs.push(cred_ref.clone());
                components.push(json!({
                    "type": "cryptographic-asset",
                    "bom-ref": cred_ref,
                    "name": cred.name,
                    "properties": [
                        { "name": "tracelamp:kind", "value": "credential" },
                        { "name": "tracelamp:storage", "value": cred.storage }
                    ]
                }));
            }
            dependencies.push(json!({
                "ref": server_ref,
                "dependsOn": child_refs
            }));
        }
        dependencies.push(json!({
            "ref": agent_ref,
            "dependsOn": server_refs
        }));
    }

    dependencies.insert(
        0,
        json!({
            "ref": "tracelamp-scan",
            "dependsOn": agent_refs
        }),
    );

    json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "bom-ref": "tracelamp-scan",
                "name": "tracelamp-scan"
            },
            "tools": {
                "components": [{
                    "type": "application",
                    "name": "tracelamp",
                    "version": env!("CARGO_PKG_VERSION")
                }]
            }
        },
        "components": components,
        "dependencies": dependencies
    })
}

fn server_component(server_ref: &str, server: &InventoryServer) -> Value {
    let mut properties = vec![json!({ "name": "tracelamp:kind", "value": "mcp-server" })];
    if !server.command.is_empty() {
        properties.push(json!({ "name": "tracelamp:command", "value": server.command }));
    }
    if !server.args.is_empty() {
        properties.push(json!({
            "name": "tracelamp:args",
            "value": server.args.join(" ")
        }));
    }
    json!({
        "type": "library",
        "bom-ref": server_ref,
        "name": server.name,
        "properties": properties
    })
}
