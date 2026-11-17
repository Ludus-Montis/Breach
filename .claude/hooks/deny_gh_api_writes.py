#!/usr/bin/env python3
"""PreToolUse hook on Bash: block any `gh api` call whose method isn't GET (Decision 191).

GitHub goes through the GitHub MCP tools; `gh` is for reads only, such as the code search cloud
sessions run with `gh api -X GET search/code -f q=...`. A deny rule can't express "fields only
with -X GET", so this hook works out each `gh api` call's method the way gh does: the last of
-X, --method or --method= (any case, any spelling pflag accepts), otherwise POST when any of -f,
-F, --field, --raw-field or --input is given, otherwise GET. Anything but GET is blocked.

It reads every command in the line: after ; & | ( and newlines, past VAR=value prefixes and
wrappers such as env, inside $(...), backticks, `sh -c '...'` and eval. A `gh api` it can't
parse (unbalanced quotes, an unknown flag) is blocked. Exit 2 blocks the call and shows stderr
to Claude; exit 0 allows it.

Known limit: a shell alias or function, or a script file that runs gh, is not seen. The token's
own permissions are the other layer.
"""

import json
import os
import re
import shlex
import sys

GUIDE = (
    "gh api may only read (GET), Decision 191. Its method is POST when it has -f, -F, --field,\n"
    "--raw-field or --input and no -X GET. Write to GitHub through the GitHub MCP tools; if the\n"
    "tool you need is missing or fails, stop and tell the owner."
)

WRAPPERS = {"env", "time", "command", "nohup", "exec", "sudo", "nice", "xargs"}
SHELLS = {"sh", "bash", "zsh", "dash"}
SHORT_BOOL = set("i")
SHORT_VALUE = {"X": "method", "f": "field", "F": "field", "H": None, "p": None, "q": None, "t": None}
LONG_BOOL = {"--include", "--paginate", "--slurp", "--silent", "--verbose"}
LONG_VALUE = {
    "--method": "method",
    "--field": "field",
    "--raw-field": "field",
    "--input": "field",
    "--header": None,
    "--preview": None,
    "--jq": None,
    "--template": None,
    "--cache": None,
    "--hostname": None,
}
OPERATOR = re.compile(r"^[;&|()\n]+$")


class Unparseable(Exception):
    pass


def tokens(text):
    # Backticks are command substitution: treat them as command boundaries.
    lex = shlex.shlex(text.replace("`", "\n"), posix=True, punctuation_chars=";&|()\n")
    lex.whitespace = " \t\r"
    lex.whitespace_split = True
    return list(lex)


def simple_commands(text):
    """Yield each simple command's words, recursing into sh -c and eval."""
    current = []
    for tok in tokens(text) + [";"]:
        if OPERATOR.match(tok):
            if current:
                yield from expand(current)
            current = []
        else:
            current.append(tok)


def expand(words):
    i = 0
    while i < len(words) and (re.match(r"^\w+=", words[i]) or name(words[i]) in WRAPPERS or words[i] == "$"):
        i += 1
    words = words[i:]
    if not words:
        return
    cmd = name(words[0])
    if cmd in SHELLS and "-c" in words[1:]:
        at = words.index("-c", 1)
        if at + 1 < len(words):
            yield from simple_commands(words[at + 1])
        return
    if cmd == "eval":
        yield from simple_commands(" ".join(words[1:]))
        return
    yield words


def name(word):
    base = os.path.basename(word.replace("\\", "/")).lower()
    return base[:-4] if base.endswith(".exe") else base


def method_of(args):
    """The method gh api would send, from the words after `api`."""
    method, fields = None, False

    def take(kind, value):
        nonlocal method, fields
        if kind == "method":
            method = value
        elif kind == "field":
            fields = True

    i = 0
    while i < len(args):
        tok = args[i]
        if tok == "--":
            break
        if tok.startswith("--"):
            flag, eq, value = tok.partition("=")
            if flag in LONG_VALUE:
                if not eq:
                    i += 1
                    if i >= len(args):
                        raise Unparseable(f"{flag} has no value")
                    value = args[i]
                take(LONG_VALUE[flag], value)
            elif flag not in LONG_BOOL:
                raise Unparseable(f"unknown flag {flag}")
        elif tok.startswith("-") and len(tok) > 1:
            chars = tok[1:]
            for j, c in enumerate(chars):
                if c in SHORT_BOOL:
                    continue
                if c not in SHORT_VALUE:
                    raise Unparseable(f"unknown flag -{c}")
                rest = chars[j + 1 :]
                if rest:
                    value = rest[1:] if rest.startswith("=") else rest
                else:
                    i += 1
                    if i >= len(args):
                        raise Unparseable(f"-{c} has no value")
                    value = args[i]
                take(SHORT_VALUE[c], value)
                break
        i += 1
    if method:
        return method.upper()
    return "POST" if fields else "GET"


def gh_api_writes(text):
    """The non-GET methods of the gh api calls in a command line."""
    found = []
    for words in simple_commands(text):
        if name(words[0]) == "gh" and len(words) > 1 and words[1] == "api":
            method = method_of(words[2:])
            if method != "GET":
                found.append(method)
    return found


def main() -> int:
    try:
        event = json.load(sys.stdin)
        command = event["tool_input"]["command"]
    except (ValueError, KeyError, TypeError) as exc:
        # Fail closed: an unreadable event must not let a command through unchecked.
        print(f"deny_gh_api_writes: could not read the event ({exc!r})", file=sys.stderr)
        return 2

    try:
        writes = gh_api_writes(command)
    except (ValueError, Unparseable) as exc:
        if re.search(r"\bgh(?:\.exe)?\s+api\b", command):
            print(f"deny_gh_api_writes: could not parse a gh command ({exc}).\n{GUIDE}", file=sys.stderr)
            return 2
        return 0

    if writes:
        print(f"Blocked gh api {', '.join(writes)}.\n{GUIDE}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
