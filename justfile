# Task runner. Install everything with `uv sync --all-extras --all-groups`, then `uv run just <recipe>`.
set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# Install the toolchain and dependencies
setup:
    uv sync --all-extras --all-groups

# Format Rust and Python
fmt:
    cargo fmt --all
    ruff format python scripts
    ruff check --fix python scripts

# Lint everything (no changes)
lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    ruff check python scripts
    ruff format --check python scripts
    mypy
    typos
    python scripts/check_style.py
    python scripts/roadmap.py check
    python scripts/required_checks.py check
    python scripts/check_yaml.py
    python scripts/check_task_ids.py
    python scripts/pin_actions.py --check
    python scripts/mutants.py check-exclusions
    python scripts/mutants.py check-slices
    python scripts/test_failures.py quarantine-check
    for f in sprints/S*.yaml; do [ ! -e "$f" ] || python scripts/sprint.py validate "$f" || exit 1; done

# Run all tests (Rust incl. golden, Python incl. golden and docs snippets)
test:
    cargo test --workspace
    pytest

# Regenerate schema snapshots, the methods reference and .claude/ from .agents/
generate:
    python scripts/schema_snapshots.py
    python scripts/gen_reference.py
    python scripts/tool_examples.py
    python scripts/sync_agents.py

# Fail if generated files are stale
check-generated:
    python scripts/schema_snapshots.py --check
    python scripts/gen_reference.py --check
    python scripts/tool_examples.py --check
    python scripts/sync_agents.py --check

# Documentation coverage (every method documented, tested, exposed)
docs-coverage:
    python scripts/docs_sync.py coverage
    python scripts/docs_sync.py claims
    python scripts/method_status.py check

# Documentation impact of this branch against main
docs-impact base="origin/main":
    python scripts/docs_sync.py impact --base {{base}}

# Regenerate CHANGELOG.md (technical, from Conventional Commits)
changelog:
    git-cliff --output CHANGELOG.md

# Whether a version can be released (policy in docs/en/project/releases.md)
release-ready version:
    python scripts/release.py ready {{version}}

# Preview the functional release notes of the next version (from changes/)
release-notes:
    towncrier build --draft --version "$(python scripts/version.py show)"

# Show or check the single-source version
version:
    python scripts/version.py check

# MCP server over stdio, driven by the MCP Inspector (needs Node.js 22.23 or later; #331)
mcp-check:
    python scripts/mcp_protocol_check.py

# Search the session journal: an issue, area:<name>, a date, or any text (#327)
journal query:
    python scripts/journal.py search "{{query}}"

# The pull request queue: behind main, in conflict or failing (#320)
pr-queue:
    python scripts/pr_queue.py list

# Translation status (English is the source)
i18n:
    python scripts/i18n.py status

# Build both documentation sites (English and Italian) in strict mode
docs: docs-coverage
    python scripts/i18n.py check
    python scripts/docs.py build

# Live preview of one language: just docs-serve it
docs-serve lang="en":
    python scripts/docs.py serve --lang {{lang}}

# The full local gate: run before every PR
check: check-test-profile lint test check-generated docs
    @echo "All checks passed."

# The gate runs every test in full: the mutants test profile samples the golden sweeps
# (crates/fairmark-core/src/test_profile.rs) and is set only by cargo-mutants
check-test-profile:
    #!/usr/bin/env bash
    set -euo pipefail
    case "${FAIRMARK_TEST_PROFILE-full}" in
      full) ;;
      *) echo "FAIRMARK_TEST_PROFILE=${FAIRMARK_TEST_PROFILE} would sample the golden sweeps: unset it to run the full gate." >&2
         exit 1 ;;
    esac

# Dependencies with newer versions available (nothing is changed)
deps-outdated:
    cargo update --dry-run --verbose 2>&1 | grep -E "Updating|Unchanged" || true
    uv tree --outdated --depth 1 | grep -E "latest" || echo "Python: all direct dependencies are current."
    # Cargo tools pinned in the workflows (install-action and its cargo install fallback);
    # Dependabot does not update these pins.
    for pin in $(grep -rhoE '(tool: |cargo install --locked )[a-z0-9-]+@[0-9][0-9.]*' .github/workflows | sed -E 's/.* //' | sort -u); do \
      name="${pin%@*}"; pinned="${pin#*@}"; \
      latest=$(cargo search "$name" --limit 1 | sed -nE "s/^$name = \"([^\"]+)\".*/\1/p"); \
      if [ "$latest" = "$pinned" ]; then echo "Tool $name $pinned: current."; \
      else echo "Tool $name $pinned: latest is ${latest:-unknown}."; fi; \
    done

# Security advisories for Rust and Python dependencies
deps-audit:
    cargo deny check advisories
    uv export --frozen --all-extras --all-groups --no-emit-project --no-hashes -o /tmp/fairmark-requirements.txt
    uvx pip-audit --strict -r /tmp/fairmark-requirements.txt

# Mutation testing of fairmark-core (cargo-mutants 27.1.0; docs/en/project/mutation-testing.md).
# Whole core, or one module: just mutants tvm | just mutants dcf/engine | just mutants tv
mutants module="":
    #!/usr/bin/env bash
    set -euo pipefail
    src=crates/fairmark-core/src
    args=(--package fairmark-core --no-shuffle --output target/mutants)
    if [ -n "{{module}}" ]; then
      if [ -f "$src/{{module}}.rs" ]; then args+=(--file "$src/{{module}}.rs")
      elif [ -d "$src/{{module}}" ]; then args+=(--file "$src/{{module}}/**/*.rs")
      else echo "No module {{module}} under $src" >&2; exit 1; fi
    fi
    status=0
    cargo mutants "${args[@]}" || status=$?
    full=""
    if [ -z "{{module}}" ]; then full="--full"; fi
    python scripts/mutants.py summary target/mutants $full
    exit "$status"

# Mutation testing of one slice of the weekly rotation (tests/mutants/budget.toml), as the
# Mutation testing workflow runs it: just mutants-slice (this week) | just mutants-slice 4
mutants-slice index="":
    #!/usr/bin/env bash
    set -euo pipefail
    index="{{index}}"
    mkdir -p target/mutants-slice
    python scripts/mutants.py slice ${index:+--index "$index"} --args-file target/mutants-slice/select.args \
      --part-file target/mutants-slice/part
    mapfile -t select < target/mutants-slice/select.args
    part=$(cat target/mutants-slice/part)
    status=0
    cargo mutants --package fairmark-core --no-shuffle "${select[@]}" \
      ${part:+--sharding round-robin --shard "$part"} --output target/mutants-slice || status=$?
    python scripts/mutants.py summary target/mutants-slice
    exit "$status"

# Mutation testing of the lines of fairmark-core changed since the merge base with a branch
mutants-diff base="origin/main":
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p target/mutants-diff
    git diff "$(git merge-base {{base}} HEAD)" -- crates/fairmark-core/src > target/mutants-diff/core.diff
    if [ ! -s target/mutants-diff/core.diff ]; then echo "No change in crates/fairmark-core/src."; exit 0; fi
    status=0
    cargo mutants --package fairmark-core --no-shuffle --in-diff target/mutants-diff/core.diff \
      --output target/mutants-diff || status=$?
    # Exit 2 (missed) and 3 (timeout) are decided by the gate below, which applies the reviewed
    # equivalent mutants; any other failure of the tool is reported as is.
    if [ "$status" -ne 0 ] && [ "$status" -ne 2 ] && [ "$status" -ne 3 ]; then exit "$status"; fi
    python scripts/mutants.py summary target/mutants-diff --gate --allow-empty --title "Mutation testing of the changed lines"

# Read-only report of worktrees, caches and disk usage; deletes nothing
# (docs/en/project/workspace-report.md): just workspace-report | just workspace-report --offline
workspace-report *args:
    python scripts/workspace_report.py {{args}}

# Check commit message, or pull request title, branch and body, against the conventions
conventions title branch body="":
    python scripts/conventions.py pr --title "{{title}}" --branch "{{branch}}" --body "{{body}}"
