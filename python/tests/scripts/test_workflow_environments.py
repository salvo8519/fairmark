"""The environment that holds the GitHub App key is used only where a branch cannot reach it.

The environment `fairmark-app` accepts deployments from `main` only. Events such as
`pull_request_target`, `issue_comment` or `issues` also run with the ref `main`, while reacting to
content written by others; a job of such a workflow must never name the environment (#372).

Every stored credential is read only by jobs of the environment that holds it, so a branch that
adds or edits a workflow cannot read it without the environment's rules: `main` only for the
App key, a protected tag for the crates.io token.
No credential is a repository secret.

These tests are a regression guard against honest mistakes: they run on the branch they check,
so a malicious branch can edit them. The protection itself is the environments' settings,
checked weekly by `maintenance.yml`, and the approval of each run.
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any

import yaml
from conftest import ROOT

WORKFLOWS = ROOT / ".github" / "workflows"
APP_ENVIRONMENT = "fairmark-app"
ALLOWED_EVENTS = {"schedule", "workflow_dispatch"}


def environment_name(job: dict[str, Any]) -> str | None:
    env = job.get("environment")
    return env.get("name") if isinstance(env, dict) else env


def jobs_with_the_app_environment() -> list[tuple[Path, dict[str, Any], str, dict[str, Any]]]:
    found = []
    for path in workflow_files():
        data = yaml.safe_load(path.read_text(encoding="utf-8"))
        # YAML 1.1 reads the key `on` as True.
        events = data.get("on", data.get(True))
        names = set(events) if isinstance(events, (dict, list)) else {events}
        for job_id, job in (data.get("jobs") or {}).items():
            if environment_name(job) == APP_ENVIRONMENT:
                found.append((path, {"events": names}, job_id, job))
    return found


def test_the_app_environment_is_used() -> None:
    names = {(p.name, job_id) for p, _, job_id, _ in jobs_with_the_app_environment()}
    assert ("prepare-release.yml", "prepare") in names
    assert ("test-health.yml", "update") in names


def test_only_scheduled_or_manual_workflows_on_main_use_the_app_environment() -> None:
    for path, meta, job_id, job in jobs_with_the_app_environment():
        where = f"{path.name}:{job_id}"
        assert meta["events"] <= ALLOWED_EVENTS, f"{where} runs on {sorted(meta['events'])}"
        assert "refs/heads/main" in str(job.get("if", "")), f"{where} is not limited to main"


SECRET_ENVIRONMENTS = {
    "FAIRMARK_APP_PRIVATE_KEY": APP_ENVIRONMENT,
    "CARGO_REGISTRY_TOKEN": "crates-io",
}
REPOSITORY_SECRETS: dict[str, str] = {}  # secret name -> the only workflow allowed to read it


SECRET_NAME = re.compile(r"secrets\s*\.\s*([A-Za-z0-9_]+)", re.IGNORECASE)
FORBIDDEN = [
    (re.compile(r"secrets\s*\[", re.IGNORECASE), "secrets[...]: a name the test cannot read"),
    (re.compile(r"tojson\s*\(\s*secrets", re.IGNORECASE), "toJSON(secrets): every secret"),
    (re.compile(r"secrets\s*:\s*inherit", re.IGNORECASE), "secrets: inherit"),
]


def workflow_files() -> list[Path]:
    return sorted([*WORKFLOWS.glob("*.yml"), *WORKFLOWS.glob("*.yaml")])


def secrets_read(node: Any) -> set[str]:
    text = yaml.safe_dump(node)
    return {name.upper() for name in SECRET_NAME.findall(text)}


def test_every_secret_is_read_only_inside_its_environment() -> None:
    seen = set()
    for path in workflow_files():
        data = yaml.safe_load(path.read_text(encoding="utf-8"))
        # A secret read at the workflow level reaches every job of the file.
        shared = secrets_read({k: v for k, v in data.items() if k != "jobs"})
        for job_id, job in (data.get("jobs") or {}).items():
            where = f"{path.name}:{job_id}"
            for name in (secrets_read(job) | shared) - {"GITHUB_TOKEN"}:
                seen.add(name)
                if name in REPOSITORY_SECRETS:
                    assert path.name == REPOSITORY_SECRETS[name], f"{where} reads {name}"
                    continue
                assert name in SECRET_ENVIRONMENTS, f"{where} reads {name}, not in any environment"
                assert environment_name(job) == SECRET_ENVIRONMENTS[name], f"{where} reads {name}"
    assert "FAIRMARK_APP_PRIVATE_KEY" in seen


def test_no_workflow_reads_secrets_in_a_form_the_test_cannot_follow() -> None:
    for path in workflow_files():
        text = path.read_text(encoding="utf-8")
        for pattern, what in FORBIDDEN:
            assert not pattern.search(text), f"{path.name} uses {what}"


def test_the_guard_sees_every_evasion() -> None:
    assert secrets_read({"run": "echo ${{ secrets.some_api_key }}"}) == {"SOME_API_KEY"}
    assert secrets_read({"run": "echo ${{ secrets . X }}"}) == {"X"}
    for text in ("${{ secrets['A'] }}", "${{ toJSON(secrets) }}", "secrets: inherit"):
        assert any(p.search(text) for p, _ in FORBIDDEN), text
