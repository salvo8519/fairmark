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

# Build the documentation site in strict mode
docs:
    zensical build --strict

# The full local gate: run before every PR
check: lint test check-generated docs
    @echo "All checks passed."
