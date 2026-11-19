#!/usr/bin/env bash
# Stop hook: the fast checks on the files changed in the session (rustfmt, clippy on the touched
# crates, ruff, mypy). Exit 2 sends the findings back to the agent, which fixes them before it
# ends its turn. The logic lives in quality_gate.py, next to this file.
command -v python3 >/dev/null 2>&1 || { echo "quality-gate hook: python3 not found, check skipped" >&2; exit 0; }
exec python3 "$(dirname "$0")/quality_gate.py"
