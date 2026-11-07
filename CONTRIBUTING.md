# Contributing to Fairmark

Thank you for helping build an open, verifiable valuation engine. Developers and valuation
professionals are equally welcome.

## Ways to contribute

- **Golden cases.** A worked example with inputs, expected outputs and its source is one of the
  most valuable contributions. Use the *Numerical discrepancy* or *Method request* issue forms.
- **Methods.** Implement a method from the roadmap (see `docs/concepts/architecture.md#adding-a-method`).
- **Docs and translations.** Methodology pages, examples, translations.

## Development setup

Requirements: [Rust](https://rustup.rs) (stable) and [uv](https://docs.astral.sh/uv/).
uv installs the right Python version for you.

```bash
uv sync --all-extras --all-groups  # creates .venv, builds the Rust extension, installs tools
cargo test --workspace             # Rust tests + golden cases
uv run pytest                      # Python tests + the same golden cases
```

The Rust extension is rebuilt automatically by `uv run` / `uv sync` when Rust sources change.
`uv.lock` is committed: dependency updates go through PRs.

Documentation (Zensical):

```bash
uv run python scripts/gen_reference.py   # regenerate the methods reference after changing a method
uv run zensical serve                    # live preview at http://localhost:8000
```

Before opening a PR: `cargo fmt --all`, `cargo clippy --workspace -- -D warnings`,
`uv run ruff check python && uv run ruff format python`, `uv run mypy`.

## Rules

1. Every PR touching a method includes golden cases and updates its page in `docs/methods/`.
2. Any change in a golden result needs the `numerical-change` label, a `method_version` bump and
   the commit trailer `Numerical-change: <method> v<N> -> v<N+1>` (the CHANGELOG is generated
   from commits). Maintainers of `crates/fairmark-core` must approve.
3. New methodological choices start as an issue labelled `needs-methodology` and are discussed
   before code is written.
4. Never commit real client data.
5. Sign off your commits (`git commit -s`) to certify the
   [Developer Certificate of Origin](https://developercertificate.org/).
