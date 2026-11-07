---
name: add-method
description: End-to-end checklist for adding a valuation method to the Fairmark engine (core, registry, schemas, Python, docs, tests). Use when implementing a new method id.
---

<!-- Generated from .agents/skills/add-method/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

1. Confirm an `Approved` spec exists and identify its method id (`area.name`).
2. Golden cases first: `/golden-case` for each acceptance criterion with numbers.
3. Core (`crates/fairmark-core/src/<area>.rs`):
   - `<Name>Input` with `Serialize, Deserialize, JsonSchema`, `#[serde(deny_unknown_fields)]`,
     doc comment on every field with units; optional fields use `#[serde(default)]` and the
     default is documented.
   - `<Name>Output` with `Serialize, JsonSchema`.
   - `pub fn <name>(input: &<Name>Input) -> Result<(<Name>Output, Explain)>` using the checked
     helpers of `crate::num`; every visible number is an explain step; conventions listed.
   - A unit type `pub struct <Name>;` implementing `Method` (id `<theme>.<name>`, version
     "1", title, description, `compute`) in the same module (ADR 0009).
   - Unit tests for edge cases and errors.
4. Registry (`registry.rs`): add one line, `<theme>::<Name>`, to the `registry!` list, in its
   theme group. Contract cases: `tests/contracts/<id>.yaml` with at least one extreme input and
   one conventions case (`crates/fairmark-core/tests/contracts.rs` fails without them).
5. `uv run just generate`: writes `schemas/<id>/v<N>.json`, updates `schemas/index.json`
   (`introduced_in: unreleased` until the next release) and the methods reference. Never edit
   an earlier version file. Version rule: if the method already exists on `main` at version N,
   any numerical change or breaking input change makes it N+1 in this PR, once (several
   changes in one PR share the bump); CI refuses a breaking change to version N.
6. Python: thin helper in `python/fairmark/<area>.py`, exported in `__init__.py`.
7. Docs: `docs/en/methods/<id>.md` (via `docs-writer`), nav entry in `zensical.toml`,
   row in `docs/en/methods/index.md`; `python scripts/docs_sync.py coverage` must pass.
8. `uv run just check`, then `code-reviewer` and `numerical-auditor`.
