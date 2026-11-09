# Contributing to Fairmark

Thank you for helping build an open, verifiable valuation engine. Developers and valuation
professionals are equally welcome.

## Ways to contribute

- **Golden cases.** A worked example with inputs, expected outputs and its source is one of the
  most valuable contributions. Use the *Numerical discrepancy* or *Method request* issue forms.
- **Methods.** Implement a method from the roadmap (see `docs/en/concepts/architecture.md#adding-a-method`).
- **Docs and translations.** Methodology pages, examples, translations.

## Development setup

Requirements: [Rust](https://rustup.rs) (stable) and [uv](https://docs.astral.sh/uv/).
uv installs the right Python version for you.

```bash
uv sync --all-extras --all-groups  # creates .venv, builds the Rust extension, installs tools
cargo test --workspace             # Rust tests + golden cases + property tests
uv run pytest                      # Python tests + the same golden cases
```

Property tests (`crates/fairmark-core/tests/properties.rs`, with `proptest`) check identities
such as NPV at the IRR being zero on generated inputs. Each property runs through a small
`check` helper with a fixed number of cases and a fixed seed, so every run checks the same
inputs; `PROPTEST_*` environment variables have no effect (a test verifies it), and no
regression files are written. Each tolerance is derived in a comment from the 28-digit decimal
precision. A failing property is investigated and reported, never loosened to pass.

Mutation testing (`cargo install --locked cargo-mutants@27.1.0`) checks that the tests of the
core detect wrong results: `uv run just mutants-diff` tests the lines of
`crates/fairmark-core/src` you changed, as CI does on every pull request, and
`uv run just mutants <module>` tests one module. Every missed mutant is killed by a new test or
recorded as equivalent after review; see
[Mutation testing](docs/en/project/mutation-testing.md).

The Rust extension is rebuilt automatically by `uv run` / `uv sync` when Rust sources change.
`uv.lock` is committed: dependency updates go through PRs.

Documentation (Zensical):

```bash
uv run python scripts/gen_reference.py   # regenerate the methods reference after changing a method
uv run zensical serve                    # live preview at http://localhost:8000
```

Before opening a PR: `cargo fmt --all`, `cargo clippy --workspace -- -D warnings`,
`uv run ruff check python && uv run ruff format python`, `uv run mypy`.

## Branches, commits and pull requests

Names follow [the conventions](docs/en/project/conventions.md) and are checked automatically:

- branch `<type>/<issue>-<slug>`, for example `feat/42-mid-year-discounting`;
- commits and pull request titles `<type>(<scope>): <subject>` (Conventional Commits), signed off;
- the pull request body links its issue with `Closes #42`.

Install the hooks once: `uv run pre-commit install` (checks formatting and commit messages).
Development uses any supported Python version, 3.12 or later
([supported versions](docs/en/project/dependencies.md#supported-python-versions)).
Check a title before opening a pull request:
`uv run just conventions "feat(dcf): add mid-year discounting" feat/42-mid-year-discounting "Closes #42"`.

## Languages

Code, commits, issues, pull requests, specs and the source documentation are in **English**.
The documentation also has an **Italian** edition (`docs/it/`, `README.it.md`): when you change
an English page, update its translation or label the PR `translation-pending`, and a
maintainer or the `translator` agent will do it. Use the terms in `i18n/glossary.yaml`.
Proposals for new documentation languages are welcome as a Discussion first.

## Release notes

If your change is visible to users, add a short fragment in `changes/` (see
`changes/README.md`): one to three sentences for a financial analyst, not for a developer.
The technical detail goes in your commit messages, which generate `CHANGELOG.md`.

## Rules

1. Every PR touching a method includes golden cases and updates its page in `docs/en/methods/`.
2. Any change in a golden result needs the `numerical-change` label, a `method_version` bump and
   the commit trailer `Numerical-change: <method> v<N> -> v<N+1>` (the CHANGELOG is generated
   from commits). Maintainers of `crates/fairmark-core` must approve.
3. Input schemas are versioned (`schemas/<method>/v<N>.json`, ADR 0009). A version present on
   `main` may only gain optional fields or documentation; removing or tightening anything needs
   the next method version. CI refuses any other change, without an override label.
4. New methodological choices start as an issue labelled `needs-methodology` and are discussed
   before code is written.
5. Never commit real client data.
6. Sign off your commits (`git commit -s`) to certify the
   [Developer Certificate of Origin](https://developercertificate.org/).
