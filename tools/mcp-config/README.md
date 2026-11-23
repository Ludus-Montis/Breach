# mcp-config: a project-scope MCP config check (study exercise)

A study exercise for the Claude Certified Architect exam, task statement 2.4 (MCP servers in
Claude Code). `example.mcp.json` is what a Breach `.mcp.json` would look like, and
`mcp_config.py` checks it the way Claude Code reads it: it expands `${VAR}` and
`${VAR:-default}`, and flags any credential written out in the file.

The example is not at the repository root on purpose. A root `.mcp.json` is project scope:
every interactive session would ask to approve its servers, and `claude -p` runs in CI would
load them without asking.

The `STUDY 2.4` line is the `Authorization` header in `example.mcp.json`, which holds a literal
token. `StudyExampleConfig` stays red until it is fixed. Run the tests from this folder with
`python3 -m unittest test_mcp_config -v`, or check a file with `python3 mcp_config.py <file>`.
