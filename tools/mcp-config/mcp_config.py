"""Checks a project-scope MCP config (`.mcp.json`) the way Claude Code reads it (task 2.4).

Claude Code expands `${VAR}` and `${VAR:-default}` in `command`, `args`, `env`, `url` and
`headers`. A variable set to the empty string counts as set, so its default is not used.
"""

import json
import os
import re

EXPANDED_FIELDS = ("command", "args", "env", "url", "headers")
_VAR = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)(?::-([^}]*))?\}")

# Shapes of real credentials. A match in a checked-in file means a secret went to git.
_SECRET = re.compile(r"(ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|sk-ant-[A-Za-z0-9-]{10,}"
                     r"|Bearer\s+(?!\$\{)\S+)")


def expand(value: str, env: dict) -> tuple[str, list[str]]:
    """Expanded string, and the names of variables that were unset with no default."""
    missing = []

    def sub(m):
        name, default = m.group(1), m.group(2)
        if name in env:            # set, even to "": the default is not used
            return env[name]
        if default is not None:
            return default
        missing.append(name)       # Claude Code loads the server and warns
        return m.group(0)

    return _VAR.sub(sub, value), missing


def _strings(node, field=None):
    """(field, string) for every string under the fields Claude Code expands."""
    if isinstance(node, str):
        yield field, node
    elif isinstance(node, list):
        for v in node:
            yield from _strings(v, field)
    elif isinstance(node, dict):
        for k, v in node.items():
            yield from _strings(v, field or k)


def literal_secrets(config: dict) -> list[str]:
    """`server.field` for each place a credential is written out instead of referenced."""
    found = []
    for name, server in config["mcpServers"].items():
        for field in EXPANDED_FIELDS:
            for _, s in _strings(server.get(field, {}), field):
                if _SECRET.search(s):
                    found.append(f"{name}.{field}")
    return found


def unset_variables(config: dict, env: dict | None = None) -> list[str]:
    env = dict(os.environ) if env is None else env
    missing = []
    for server in config["mcpServers"].values():
        for field in EXPANDED_FIELDS:
            for _, s in _strings(server.get(field, {}), field):
                missing += expand(s, env)[1]
    return sorted(set(missing))


if __name__ == "__main__":
    import sys
    cfg = json.load(open(sys.argv[1] if len(sys.argv) > 1 else ".mcp.json"))
    leaks = literal_secrets(cfg)
    print("literal secrets:", leaks or "none")
    print("unset, no default:", unset_variables(cfg) or "none")
    sys.exit(1 if leaks else 0)
