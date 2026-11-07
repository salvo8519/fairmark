# AGENTS.md

Instructions for AI coding agents (Claude Code, Codex, Copilot, Cursor, …) working on this
repository. Humans: see `CONTRIBUTING.md`. Users of Fairmark (not developers): see
`skills/fairmark/SKILL.md`.

## What this project is
Fairmark is a deterministic valuation engine (Rust core) with thin adapters: Python SDK,
CLI, MCP server, function-calling tools. It values private companies (and later real assets)
with exact decimal arithmetic and a full audit trail. Read `specs/constitution.md` before
changing anything: its principles override convenience.

## Repository map
| Path | What lives there |
| --- | --- |
| `crates/fairmark-core/` | ALL financial logic. Methods, decimal math, explain, registry, errors. No I/O. |
| `crates/fairmark-cli/` | `fairmark` binary (methods / schema / run / verify). Adapter only. |
| `crates/fairmark-py/` | PyO3 bridge: 3 functions, JSON strings across the boundary. |
| `python/fairmark/` | Pythonic API, `tools.py` (function calling), `mcp_server.py`. Adapters only. |
| `tests/golden/` | Golden cases shared by Rust and Python. Human-reviewed. |
| `schemas/` | Committed JSON Schema snapshots of every method input (the "migrations"). |
| `specs/` | Spec-driven development: constitution, templates, one folder per feature. |
| `docs/` | Documentation site (Zensical). `docs/reference/methods.md` is generated. |
| `roadmap/roadmap.yaml` | Milestones → epics → tasks, mirrored to GitHub issues. |
| `.agents/` | Canonical agent roles (`agents/`) and skills (`skills/`). `.claude/` is generated from it. |

## How work is done here: spec-driven
1. Find or write the spec: `specs/NNNN-*/spec.md`. **No method code without an `Approved`
   spec.** Tooling, docs and refactors don't need one.
2. Read `plan.md` and `tasks.md`. Work one task at a time, **test first**.
3. Cite requirement ids in commits and PRs: `feat(dcf): FCFF build-up [SPEC-0001 R2]`.
4. Tick tasks in `tasks.md` only when `just check` passes.

## Hard rules
- `rust_decimal::Decimal` for every value. **Never** `f32`/`f64` in a calculation path; in
  Python never `float` for money or rates (use `Decimal` or strings).
- Financial logic only in `fairmark-core`. Adapters translate, they never compute.
- New input types: `Serialize + Deserialize + JsonSchema`, `#[serde(deny_unknown_fields)]`,
  a doc comment on every field stating units (rates are decimal fractions).
- Every calculation step that produces a number the user sees goes into `Explain`; every
  convention that changes a number goes into `explain.conventions`.
- Errors: add a variant to `FairmarkError` with the next free stable code, document it in
  `docs/reference/errors.md`. Never reuse or renumber codes.
- **Never modify expected values of an existing golden case.** If a result must change it is
  a `numerical-change`: stop and ask the maintainer. New golden cases need an independent
  source noted in the file.
- Never commit real client data. Anonymise or reconstruct.
- Conventional Commits for messages and PR titles: `feat|fix|docs|refactor|test|chore|ci|perf(scope): …`.
- Sign off commits (`git commit -s`, DCO).

## House style (enforced by CI)
- **No icons, emoji, pictograms or decorative symbols** in any file: documentation, README,
  code comments, commit messages, PR descriptions, issue text and generated output. Use words
  ("Available", "Planned", "Note:") instead. Typographic symbols of finance and maths
  (× − Σ → · ≤ ≥ €) are fine. `scripts/check_style.py` fails the build otherwise.
- Corporate, precise English: short sentences, explicit units, no marketing superlatives,
  no exclamation marks.
- Diagrams use the brand palette (`docs/project/brand.md`) and plain text labels.

## Agents and skills
Roles and workflows live in `.agents/` (see `.agents/README.md`): `architect`, `spec-writer`,
`test-engineer`, `core-engineer`, `docs-writer`, `code-reviewer`, `numerical-auditor`,
`security-reviewer`, `ci-investigator`, `release-manager`; skills `spec-new`,
`implement-spec`, `add-method`, `golden-case`, `adr-new`, `pre-pr`, `fix-ci`, `release`.
Edit them in `.agents/` only; `uv run just generate` mirrors them to `.claude/`.

## Commands
Install [`uv`](https://docs.astral.sh/uv/) and Rust stable. `just` is installed by uv.

| Goal | Command |
| --- | --- |
| Set up everything | `uv sync --all-extras --all-groups` |
| Full local gate (run before every PR) | `uv run just check` |
| Format everything | `uv run just fmt` |
| Rust tests incl. golden | `cargo test --workspace` |
| Python tests incl. golden and docs snippets | `uv run pytest` |
| Regenerate schemas, methods reference, `.claude/` | `uv run just generate` |
| Docs preview | `uv run zensical serve` |

## Adding a method (checklist)
1. Spec approved → golden cases from an independent source in `tests/golden/<method>/`.
2. `crates/fairmark-core/src/<area>.rs`: input/output types + function returning
   `Result<(Output, Explain)>`.
3. Register in `registry.rs` (`METHODS`, `input_schema`, `run`).
4. `uv run just generate` → updates `schemas/` and `docs/reference/methods.md`.
5. Python helper in `python/fairmark/<area>.py`; methodology page in `docs/methods/`.
6. CHANGELOG entry. `uv run just check`.

## Review standards (what reviewers and CI check)
See `.agents/agents/code-reviewer.md` and `.agents/agents/numerical-auditor.md`. In short:
constitution compliance, decimal-only arithmetic, explain completeness, schema docs, golden
cases with independent sources, no adapter logic, tests for every acceptance criterion.
