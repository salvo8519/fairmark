#!/usr/bin/env bash
# Stop hook: before Claude finishes, make sure code changes came with their documentation.
# Exit 2 sends the message back to Claude, which then updates the docs (docs-writer agent).
set -uo pipefail
active=$(python3 -c 'import json,sys; print(json.load(sys.stdin).get("stop_hook_active", False))' 2>/dev/null || echo False)
[ "$active" = "True" ] && exit 0   # already continuing because of this hook: avoid loops
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
python3 scripts/docs_sync.py impact --worktree --hook
