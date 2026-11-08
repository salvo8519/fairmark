#!/usr/bin/env bash
# PreToolUse hook: block edits to golden cases that already exist on the base branch.
# The logic lives in protect_golden.py, next to this file; it reads the hook JSON on stdin.
set -euo pipefail
exec python3 "$(dirname "$0")/protect_golden.py"
