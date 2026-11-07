# SPEC-NNNN — Tasks

Rules: test first; one task per commit; `[P]` tasks touch disjoint files and can run in
parallel; tick a task only when `just check` passes.

## Tests (write first, they must fail)
- [ ] T1 [P] Golden cases for AC1 (`tests/golden/...`)
- [ ] T2 [P] Error-case unit tests for AC2

## Implementation
- [ ] T3 Input/output types with JsonSchema + docs on every field
- [ ] T4 Calculation with explain steps and conventions
- [ ] T5 Registry entry + regenerate schemas and reference

## Adapters and docs
- [ ] T6 [P] Python helper
- [ ] T7 [P] Methodology page `docs/methods/...`
- [ ] T8 `uv run just generate` and docs coverage green (`uv run just docs`)

## Verification
- [ ] T9 `numerical-auditor` review, `code-reviewer` review, all AC ticked in spec.md
