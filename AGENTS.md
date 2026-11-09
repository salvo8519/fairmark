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
| `crates/fairmark-py/` | PyO3 bridge: 5 functions, JSON strings across the boundary. |
| `python/fairmark/` | Pythonic API, `tools.py` (function calling), `mcp_server.py`. Adapters only. |
| `tests/golden/` | Golden cases shared by Rust and Python. Human-reviewed. |
| `schemas/` | Versioned JSON Schema snapshots: `<method>/v<N>.json` (input) and `<method>/v<N>.output.json` (output) for every method version, `index.json` with the history (the "migrations"). Earlier versions are immutable. |
| `specs/` | Spec-driven development: constitution, templates, one folder per feature. |
| `docs/` | Documentation: `en/` (source), `it/` (translation), `_shared/` (assets). References are generated. |
| `i18n/` | Language editions configuration and the translation glossary. |
| `roadmap/roadmap.yaml` | Milestones → epics → tasks with their issue and spec; checked and synchronised by `scripts/roadmap.py`. |
| `.agents/` | Canonical agent roles (`agents/`) and skills (`skills/`). `.claude/` is generated from it. |
| `sprints/` | Sprint plans (`SNN.yaml`) used by the orchestrator. |
| `.agents/journal/` | Session handover notes, one file per branch and day. |

## Sessions and project memory
- **Start** every session with the `session-start` skill (or
  `uv run python scripts/project_state.py`): branch, issue, journal, pull requests, CI, sprint
  and the issues that can run in parallel.
- **End** every session with `session-end`: gate, journal entry in `.agents/journal/`,
  release note, commit, push and a complete pull request. Never leave work only on a laptop.
- **Review the queue** with `pr-triage`. Agents comment and recommend; only the maintainer
  approves and merges.

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
- Method versions: bump on any numerical change, breaking input change or breaking output
  change (an output field removed, renamed, retyped, made nullable or no longer always present;
  a new output field is additive, ADR 0014 G8). Output types derive `JsonSchema` with a doc
  comment stating units. Each version keeps `schemas/<method>/v<N>.json` and
  `v<N>.output.json`; earlier versions are never edited (CI checks).
- Errors: a new meaning gets a new variant of `FairmarkError` with the next free stable code;
  a variant that only adds the location of an existing condition (for example its JSON
  pointer) keeps that condition's code: one condition, one code. Document both in
  `docs/en/reference/errors.md`. Never reuse or renumber codes.
- **Never modify expected values of an existing golden case.** If a result must change it is
  a `numerical-change`: stop and ask the maintainer. New golden cases need an independent
  source noted in the file.
- **An approval covers only the golden cases it names.** When an approved change would also
  modify golden cases outside the scope named in its approval (inputs, expected values or
  expected names), stop and ask the maintainer to extend the approval first. Never bypass the
  protection of existing golden files, by any means.
- **Code identifiers are always English**, using the term of international financial practice
  (method ids, fields, enum values, types, messages). Italian names of Italian methods appear
  only in the Italian documentation and in `i18n/glossary.yaml`; an internationally recognised
  acronym (for example `mixed.uec`) may be kept.
- Never commit real client data. Anonymise or reconstruct.
- Names follow `docs/en/project/conventions.md`, enforced by `scripts/conventions.py`:
  branch `<type>/<issue>-<slug>`; commit and pull request title `<type>(<scope>): <subject>`
  (Conventional Commits, scope required); pull request body `Closes #<issue>`; squash merge.
- New dependencies and updates follow `docs/en/project/dependencies.md`.
- **Roadmap as code.** Create issues with `python scripts/roadmap.py new` (it sets the
  milestone, the labels and the entry in `roadmap/roadmap.yaml`); a pull request may only close
  an issue that is in the roadmap, and every spec must be referenced by a roadmap task (CI).
  `roadmap.py sync` copies issue states from GitHub; a scheduled workflow reports differences.
  When a request adds, changes, moves or drops planned work, propose the roadmap change first
  (skill `roadmap-update`) and apply it after the maintainer confirms.
- Sign off commits (`git commit -s`, DCO).

## House style (enforced by CI)
- **No icons, emoji, pictograms or decorative symbols** in any file: documentation, README,
  code comments, commit messages, PR descriptions, issue text and generated output. Use words
  ("Available", "Planned", "Note:") instead. Typographic symbols of finance and maths
  (× − Σ → · ≤ ≥ €) are fine. `scripts/check_style.py` fails the build otherwise.
- Corporate, precise English: short sentences, explicit units, no marketing superlatives,
  no exclamation marks.
- Diagrams use the brand palette (`docs/en/project/brand.md`) and plain text labels.

## Languages
- **Repository language: English.** Code, comments, commit messages, PR and issue text, specs,
  ADRs and the source documentation (`docs/en/`, `README.md`) are written in English.
- **Conversation language: the contributor's choice.** Personal preferences go in
  `CLAUDE.local.md` (not committed; see `CLAUDE.local.md.example`). The `commit` skill shows the
  English message with a translation for review; `explain-changes` explains changes in any
  language; `onboarding` guides newcomers in their language.
- **Documentation editions: English (source) and Italian.** Translations live in `docs/it/`
  and `README.it.md`, each tracking the hash of its English source. `scripts/i18n.py check`
  fails when a required translation is stale or missing; the `translate` skill and the
  `translator` agent fix it, using `i18n/glossary.yaml`. New languages are added only by the
  maintainers.

## Agents and skills
Roles and workflows live in `.agents/` (see `.agents/README.md`): `architect`, `spec-writer`,
`test-engineer`, `core-engineer`, `docs-writer`, `code-reviewer`, `numerical-auditor`,
`security-reviewer`, `ci-investigator`, `release-manager`, `sprint-planner`, `translator`; skills
`spec-new`, `implement-spec`, `add-method`, `golden-case`, `adr-new`, `pre-pr`, `fix-ci`,
`release`, `plan-sprint`, `run-sprint`, `commit`, `explain-changes`, `translate`, `onboarding`,
`session-start`, `session-end`, `pr-triage`, `roadmap-update`. Sprints: see `sprints/README.md`.
Edit them in `.agents/` only; `uv run just generate` mirrors them to `.claude/`.

## Documentation stays in sync automatically
- Generated, never hand-edited: `schemas/`, `docs/en/reference/methods.md`, the Python API
  reference (docstrings), `.claude/` and `CHANGELOG.md` (git-cliff, from Conventional Commits).
- `scripts/docs_sync.py coverage`: every method has a methodology page, nav entry, index row,
  golden cases and a Python helper.
- `scripts/docs_sync.py impact`: a change to a method, error codes, CLI, MCP or function
  calling must come with the matching documentation change. Runs in pre-commit, CI and as a
  Claude Code Stop hook. Waive only with the PR label `no-docs-needed` or the commit trailer
  `Docs-impact: none`.
- Numerical changes carry the commit trailer `Numerical-change: <method> v<N> -> v<N+1>`; the
  CHANGELOG lists them first.

## Versions, releases and project memory
- **Releases**: each milestone names the release that ships it (`roadmap/roadmap.yaml`);
  `python scripts/release.py ready <version>` decides whether a version can be released
  (`docs/en/project/releases.md`). The maintainer decides when to release.
- **One version**: `[workspace.package] version` in `Cargo.toml`. Everything else derives from
  it; `scripts/version.py check` verifies it in CI, `scripts/version.py bump` changes it.
- **Two audiences, two documents**:
  - `CHANGELOG.md`, technical, generated by git-cliff from Conventional Commits: precise
    context for agents and developers;
  - `RELEASE_NOTES.md`, functional, built by towncrier from fragments in `changes/`, written
    for analysts in plain language. Every user-facing PR adds one fragment
    (`changes/README.md`); CI checks it (label `no-release-note` otherwise).
- **Context for agents**: before working on an area, read the relevant entries of
  `CHANGELOG.md`, the unreleased fragments in `changes/`, the spec and the ADRs. The docs site
  publishes `llms.txt` and `llms-full.txt` for external agents.

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
| Translation status | `uv run just i18n` |
| Project state for a new session | `uv run python scripts/project_state.py` |
| Outdated dependencies / advisories | `uv run just deps-outdated` / `uv run just deps-audit` |
| Version check / preview of release notes | `uv run just version` / `uv run just release-notes` |
| Build both documentation sites | `uv run just docs` |
| Docs preview (English or Italian) | `uv run just docs-serve` / `uv run just docs-serve it` |

## Adding a method (checklist)
1. Spec approved → golden cases from an independent source in `tests/golden/<method>/`.
2. `crates/fairmark-core/src/<theme>.rs`: input/output types and a unit type implementing the
   `Method` trait (ADR 0009).
3. Add one line to the registry list in `registry.rs`.
4. `uv run just generate` → writes `schemas/<method>/v<N>.json` and `v<N>.output.json`, updates
   `schemas/index.json` and `docs/en/reference/methods.md`. A breaking input or output change
   needs a new method version.
5. Python helper in `python/fairmark/<area>.py`; methodology page in `docs/en/methods/`.
6. Conventional Commit messages (the CHANGELOG is generated by git-cliff). `uv run just check`.
7. The method is `preview` (ADR 0012): list it in `specs/method-evidence.yaml`. Promotion to
   `available` is one line in `AVAILABLE` (`registry.rs`), approved by the maintainer and
   checked by `scripts/method_status.py` against `docs/en/project/releases.md`.

## Review standards (what reviewers and CI check)
Every finding of a review or an audit (blocking, should fix and nit) is fixed in the same pull
request. A follow-up issue is opened only for new scope, and only with the maintainer's
approval; a defect is never deferred. A change that alters computed results goes through the
numerical-change path (its own issue, spec, version bump and audit), which is not a deferral.

### Effort proportional to risk
Reviews and audits are expensive. Their cost is paid in the project's agent budget, so effort
follows risk. Classify every pull request before work starts and state the tier in its body.

| Tier | Pull requests | Gate before merge |
| --- | --- | --- |
| Small | Documentation, roadmap, tooling, CI configuration, and fixes that change no computed number, contract, schema or error code | `uv run just check`, green CI, and the orchestrator reads the diff. No review subagent. |
| Substantial | Method logic, contracts (input and output schemas, envelope, errors), adapters | One `code-reviewer` pass on the whole branch. `security-reviewer` only when dependencies, workflows or permissions change. |
| Numerical change | Results or golden cases change, a method version is bumped | `code-reviewer` and `numerical-auditor`, before merge. |

- **One review round.** Every finding is fixed; the orchestrator checks each fix against the
  list of findings. A second review pass is run only when the first found a blocking defect in
  logic, never for nits or wording.
- **Scheduled audit.** The `numerical-auditor` audits `main` once a week and at the end of every
  milestone: all golden cases against an independent reference, plus a seeded random sample.
  It does not run on each pull request, except for the numerical-change tier.
- **Test first where it pays.** Acceptance criteria of a spec, golden cases and bug fixes (a
  failing test that reproduces the bug comes first). Documentation, tooling and refactors
  already covered by tests do not need new tests written first.
- **Agent budget.** At most three agents run in parallel. Each brief is self-contained and names
  the files to read; agents do not explore the repository at large. No subagent for a task the
  orchestrator can finish in a few steps. Resume a stopped agent instead of starting a new one.
  Plan work in branches of meaningful size rather than many small rounds.

See `.agents/agents/code-reviewer.md` and `.agents/agents/numerical-auditor.md`. In short:
constitution compliance, decimal-only arithmetic, explain completeness, schema docs, golden
cases with independent sources, no adapter logic, tests for every acceptance criterion.
