---
name: add-method
description: End-to-end checklist for adding a valuation method to the Fairmark engine (core, registry, schemas, Python, docs, tests). Use when implementing a new method id.
---

1. Confirm an `Approved` spec exists and identify its method id (`area.name`).
2. Golden cases first: `/golden-case` for each acceptance criterion with numbers.
3. Core (`crates/fairmark-core/src/<area>.rs`):
   - `<Name>Input` with `Serialize, Deserialize, JsonSchema`, `#[serde(deny_unknown_fields)]`,
     doc comment on every field with units; optional fields use `#[serde(default)]` and the
     default is documented.
   - `<Name>Output` with `Serialize, JsonSchema`.
   - `pub fn <name>(input: &<Name>Input) -> Result<(<Name>Output, Explain)>` using checked
     decimal ops; every visible number is an explain step; conventions listed.
   - Unit tests for edge cases and errors.
4. Registry (`registry.rs`): add to `METHODS` (id, title, description, version "1"),
   `input_schema` and `run`.
5. `uv run just generate` (schemas + methods reference).
6. Python: thin helper in `python/fairmark/<area>.py`, exported in `__init__.py`.
7. Docs: `docs/methods/<id>.md` (via `docs-writer`), nav entry in `zensical.toml`,
   row in `docs/methods/index.md`; `python scripts/docs_sync.py coverage` must pass.
8. `uv run just check`, then `code-reviewer` and `numerical-auditor`.
