"""The Bash hook that keeps agents away from run approvals, secrets and environments (#376)."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys

import pytest
from conftest import ROOT

HOOK = ROOT / ".claude" / "hooks" / "protect_settings.py"
spec = importlib.util.spec_from_file_location("protect_settings", HOOK)
hook = importlib.util.module_from_spec(spec)  # type: ignore[arg-type]
spec.loader.exec_module(hook)  # type: ignore[union-attr]

BLOCKED = [
    "gh api -X POST repos/o/r/actions/runs/1/pending_deployments -f state=approved",
    "gh api repos/o/r/environments/anthropic",
    "gh api -X PUT repos/o/r/environments/anthropic -F can_admins_bypass=true",
    "gh api repos/o/r/environments/a/deployment_protection_rules",
    "gh api repos/o/r/actions/secrets",
    "gh api repos/o/r/actions/variables/FAIRMARK_APP_ID",
    "gh api repos/o/r/rulesets/1",
    "gh api -X DELETE repos/o/r/branches/main/protection",
    "gh secret set ANTHROPIC_API_KEY",
    "gh -R o/r secret list",
    "gh variable set FAIRMARK_APP_ID --body 1",
    "gh ruleset list",
    "gh run approve 123",
    "curl -X POST https://api.github.com/repos/o/r/actions/runs/1/PENDING_DEPLOYMENTS",
]
ALLOWED = [
    "gh api repos/o/r/pulls/3",
    "gh pr view 3",
    "gh run view 123 --log-failed",
    "git commit -m 'docs: describe secret scanning'",
    "grep -n secrets .github/workflows/ci.yml",
    "uv run pytest python/tests/scripts/test_workflow_environments.py",
    "gh issue comment 5 --body 'turn on secret scanning'",
    "gh pr create --title 'ci(ci): secret in an environment' --body x",
]


@pytest.mark.parametrize("command", BLOCKED)
def test_blocked(command: str) -> None:
    assert hook.reason_to_block(command) is not None


@pytest.mark.parametrize("command", ALLOWED)
def test_allowed(command: str) -> None:
    assert hook.reason_to_block(command) is None


def run_hook(command: str) -> subprocess.CompletedProcess[str]:
    payload = json.dumps({"tool_name": "Bash", "tool_input": {"command": command}})
    return subprocess.run(
        [sys.executable, str(HOOK)], input=payload, capture_output=True, text=True, check=False
    )


def test_the_hook_exits_2_with_a_reason_and_0_otherwise() -> None:
    blocked = run_hook(BLOCKED[0])
    assert blocked.returncode == 2
    assert "pending_deployments" in blocked.stderr and "maintainer" in blocked.stderr
    assert run_hook(ALLOWED[0]).returncode == 0


def test_the_hook_is_registered_for_bash() -> None:
    settings = json.loads((ROOT / ".claude" / "settings.json").read_text(encoding="utf-8"))
    commands = [
        h["command"]
        for entry in settings["hooks"]["PreToolUse"]
        if entry["matcher"] == "Bash"
        for h in entry["hooks"]
    ]
    assert any(c.endswith("/.claude/hooks/protect-settings.sh") for c in commands)
