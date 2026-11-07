# Contributing to Fairmark

Thank you for helping build an open, verifiable valuation engine. Developers and valuation
professionals are equally welcome.

## Ways to contribute

- **Golden cases.** A worked example with inputs, expected outputs and its source is one of the
  most valuable contributions. Use the *Numerical discrepancy* or *Method request* issue forms.
- **Methods.** Implement a method from the roadmap (see `docs/concepts/architecture.md#adding-a-method`).
- **Docs and translations.** Methodology pages, examples, translations.

## Development setup

```bash
# Rust (stable) and Python ≥ 3.10 required
python -m venv .venv && source .venv/bin/activate
pip install -e ".[dev,mcp]"        # builds the Rust extension via maturin
cargo test --workspace             # Rust tests + golden cases
pytest                             # Python tests + the same golden cases
```

Documentation (Zensical):

```bash
pip install -e ".[docs]"
python scripts/gen_reference.py   # regenerate the methods reference after changing a method
zensical serve                    # live preview at http://localhost:8000
```

Before opening a PR: `cargo fmt --all`, `cargo clippy --workspace -- -D warnings`,
`ruff check python && ruff format python`, `mypy`.

## Rules

1. Every PR touching a method includes golden cases and updates its page in `docs/methods/`.
2. Any change in a golden result needs the `numerical-change` label, a `method_version` bump and
   a CHANGELOG entry. Maintainers of `crates/fairmark-core` must approve.
3. New methodological choices start as an issue labelled `needs-methodology` and are discussed
   before code is written.
4. Never commit real client data.
5. Sign off your commits (`git commit -s`) to certify the
   [Developer Certificate of Origin](https://developercertificate.org/).
