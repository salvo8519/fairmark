"""PreToolUse hook on Bash: agents never approve runs or change secrets and environments (#376).

The environment `anthropic` gives the AI key to a run only after the maintainer approves it.
An agent session often uses the maintainer's GitHub login, and that login can approve a
waiting run through the API, or change the environment's rules. This hook blocks every
command that would do so: the approval and the settings stay a human act in the GitHub web
interface.

Blocked: GitHub API paths for pending deployments, environments, deployment protection rules,
Actions secrets and variables, rulesets and branch protection; `gh secret`, `gh variable`,
`gh ruleset` and `gh run` approvals. Reading is blocked too: an agent has no need for them,
and one rule is easier to trust than a list of methods.

It guards against an agent that was misled, not against a person at the keyboard; the
maintainer's token permissions are the second line (docs/en/project/maintainers.md).
Exit status 2 blocks the command; the message on stderr is shown to the agent.
"""

from __future__ import annotations

import json
import re
import sys

PATTERNS = [
    r"pending_deployments",
    r"/environments\b",
    r"deployment_protection_rules",
    r"deployment-protection-rules",
    r"/actions/secrets\b",
    r"/actions/variables\b",
    r"/dependabot/secrets\b",
    r"/rulesets\b",
    r"/branches/[^ ]*/protection\b",
    # gh, then global flags with an optional value (-R owner/repo), then the subcommand.
    r"\bgh\s+(?:-\S+\s+(?:[^-\s]\S*\s+)?)*(?:secret|variable|ruleset)\b",
    r"\bgh\s+run\s+(?:approve|rerun\s+--approve)",
]
BLOCKED = re.compile("|".join(PATTERNS), re.IGNORECASE)


def reason_to_block(command: str) -> str | None:
    match = BLOCKED.search(command)
    return match.group(0) if match else None


def main() -> int:
    data = json.load(sys.stdin)
    command = (data.get("tool_input") or {}).get("command") or ""
    found = reason_to_block(command)
    if found is None:
        return 0
    sys.stderr.write(
        f"Blocked: the command touches '{found}'. Approving workflow runs and changing secrets, "
        "variables, environments or rulesets is done only by the maintainer in the GitHub web "
        "interface (docs/en/project/maintainers.md). Ask the user.\n"
    )
    return 2


if __name__ == "__main__":
    sys.exit(main())
