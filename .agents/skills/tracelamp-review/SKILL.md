---
name: tracelamp-review
description: Review one selected workspace with TraceLamp. Scan, explain findings, and propose fixes without editing files or executing repository contents.
---

# TraceLamp review

Review the selected workspace. Do not edit files. Do not run the repository, its package scripts, or its tools. Do not start `tracelamp mcp serve --transport http`.

## Instructions

1. Use the TraceLamp stdio MCP server. The tools are `scan_repo`, `list_findings`, `explain_finding`, and `agent_inventory`.
2. Pass the selected workspace as `path`. It must already sit inside `TRACELAMP_SCAN_ROOT`. If the server rejects the path, stop and say the root does not contain that directory.
3. Read the findings. Call `explain_finding` with each `rule_id` you intend to discuss.
4. Propose fixes as text in the reply. Name the file, the rule id, and the change you would make. Do not apply that change.
5. Tell the user that findings and short snippets were returned to Warp and may be included in the model context. Do not repeat secret values. Header values and credential values are not part of the inventory. If a snippet still contains a secret, omit that snippet from the reply.

## Examples

- "Review this checkout with tracelamp-review."
- "/tracelamp-review explain the high findings only"
