#!/usr/bin/env bash
# PreToolUse hook: block edits to EXISTING golden cases (new cases are allowed).
set -euo pipefail
file=$(python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("tool_input",{}).get("file_path",""))')
case "$file" in
  */tests/golden/*)
    if [ -f "$file" ]; then
      echo "Blocked: $file is an existing golden case. Changing expected values is a numerical-change and needs the maintainer's review. Ask the user, or add a new case instead." >&2
      exit 2
    fi ;;
esac
exit 0
