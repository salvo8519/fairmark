#!/usr/bin/env bash
# PostToolUse hook: format the file Claude just edited.
set -euo pipefail
file=$(python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("tool_input",{}).get("file_path",""))')
[ -z "$file" ] || [ ! -f "$file" ] && exit 0
case "$file" in
  *.rs) rustfmt --edition 2021 "$file" 2>/dev/null || true ;;
  *.py) (uv run --quiet ruff format "$file" && uv run --quiet ruff check --fix --quiet "$file") >/dev/null 2>&1 || true ;;
esac
exit 0
