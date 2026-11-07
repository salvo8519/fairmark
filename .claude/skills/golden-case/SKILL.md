---
name: golden-case
description: Author a golden test case for a Fairmark method with expected values from an independent source. Use when adding coverage for a method or an acceptance criterion.
---

<!-- Generated from .agents/skills/golden-case/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

1. Choose the source of truth, in order of preference: a practitioner model supplied by the
   maintainer; a published worked example re-derived by hand; an independent reference
   implementation in `scripts/reference_values.py` (Python `decimal`, precision 50, written
   from the formula, not from the Rust code).
2. Create `tests/golden/<method>/<kebab-case-name>.yaml`:
   ```yaml
   # Source: <where the expected values come from>
   method: <method id>
   description: <one line>
   input: { ... }            # amounts and rates as strings, e.g. "0.085"
   expect: { field: "value" }
   tolerance: "0.000000001"
   ```
3. Use only anonymised or synthetic data.
4. Run `cargo test -p fairmark-core --test golden` and `uv run pytest python/tests/test_golden.py`.
5. Existing golden files are protected by a hook: never edit them; propose a
   `numerical-change` to the maintainer instead.
