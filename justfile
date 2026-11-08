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
    python scripts/pin_actions.py --check
    for f in sprints/S*.yaml; do [ -e "$f" ] && python scripts/sprint.py validate "$f"; done; true

# Run all tests (Rust incl. golden, Python incl. golden and docs snippets)
test:
    cargo test --workspace
    pytest

# Regenerate schema snapshots, the methods reference and .claude/ from .agents/
generate:
    python scripts/schema_snapshots.py
    python scripts/gen_reference.py
    python scripts/sync_agents.py

# Fail if generated files are stale
check-generated:
    python scripts/schema_snapshots.py --check
    python scripts/gen_reference.py --check
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
check: lint test check-generated docs
    @echo "All checks passed."

# Dependencies with newer versions available (nothing is changed)
deps-outdated:
    cargo update --dry-run --verbose 2>&1 | grep -E "Updating|Unchanged" || true
    uv tree --outdated --depth 1 | grep -E "latest" || echo "Python: all direct dependencies are current."

# Security advisories for Rust and Python dependencies
deps-audit:
    cargo deny check advisories
    uv export --frozen --all-extras --all-groups --no-emit-project --no-hashes -o /tmp/fairmark-requirements.txt
    uvx pip-audit --strict -r /tmp/fairmark-requirements.txt

# Check commit message, or pull request title, branch and body, against the conventions
conventions title branch body="":
    python scripts/conventions.py pr --title "{{title}}" --branch "{{branch}}" --body "{{body}}"
