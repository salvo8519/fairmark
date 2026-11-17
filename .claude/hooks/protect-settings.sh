#!/usr/bin/env bash
# PreToolUse hook on Bash: block approvals of workflow runs and changes to secrets, variables,
# environments and rulesets (#376). The logic lives in protect_settings.py, next to this file.
set -euo pipefail
exec python3 "$(dirname "$0")/protect_settings.py"
