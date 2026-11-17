# 2026-11-17 — sprint/S01/335-schema-manifests-adr

- Issue: #335 Per-method schema manifests instead of one shared index rewritten by every lane
- Pull request: the ADR only (S01 stops there); the implementation is planned after approval
- Spec: none; ADR 0017 (proposed)

## Done
- ADR 0017 proposed: one manifest per method (`schemas/<method>/index.json`), the global
  history assembled by `schema_history()` and published, not committed; immutability checked
  per method; migration with no snapshot or hash changed; methods reference and status tables
  built at documentation time; README lists themes; tool examples per method; the merge order
  kept only for the `agents` group.
- `zensical.toml` joined the files every lane may change (PR #381), for navigation entries.

## State at the end of the session
- The ADR waits for the maintainer's decision; issue #335 stays open.

## Decisions taken
- Rejected: a committed global index rewritten by a check on `main` (needs a workflow writing
  to `main`, which the ruleset and #372 keep closed).

## Open questions for the maintainer
- Accept ADR 0017.
- README: themes only (proposed), or keep the per-method table and accept its conflicts.

## Next steps
1. #208: ADR on the sources policy (maintainer approves).
