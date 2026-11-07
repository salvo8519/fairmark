# SPEC-0008 — Plan

Status: active (SPEC-0008 approved on 2026-11-07); changes to the spec are reflected here first.

## Design

### Structural decision (ADR 0010, before any code)
ADR 0009 states that composition between methods goes through typed Rust calls, not JSON. That
holds when a method composes known methods (DCF using `coc`). Generic analysis composes an
inner method chosen at run time, so it must go through the registry on JSON values. ADR 0010
records this as a second, bounded form of composition: allowed only for the `analysis` theme,
inside `fairmark-core`, one level deep (R2), with the inner id and version recorded (R1). The
`architect` writes it; the maintainer accepts it before T11 (`tasks.md`).

### Module layout in `crates/fairmark-core`
| File | Content |
| --- | --- |
| `src/analysis/mod.rs` | Shared input pieces (`method`, `base_input`), nested-analysis check, evaluation cap, `InnerMethod {id, version}`, per-cell outcome type |
| `src/analysis/pointer.rs` | Resolve and set RFC 6901 pointers on `serde_json::Value`, decimal checks (R3, R4) |
| `src/analysis/goal_seek.rs` | `GoalSeek` method (R10 to R14) |
| `src/analysis/sensitivity.rs` | `Sensitivity` method (R15 to R17) |
| `src/analysis/scenarios.rs` | `Scenarios` method (R18, R19) |
| `src/analysis/tornado.rs` | `Tornado` method (R20, R21) |
| `src/solver.rs` | `brent`: generic, `Fn(Decimal) -> Result<Decimal>`, no knowledge of methods; reusable by later methods |
| `src/registry.rs` | `pub(crate) fn evaluate(id, &Value) -> Result<Evaluation>`; four lines in the registry list |
| `src/error.rs` | Five variants, `FM-E013` to `FM-E017` (provisional) |

Type paths such as `fairmark_core::analysis::goal_seek::GoalSeek` satisfy the ADR 0009 test
(the path starts with `fairmark_core::analysis::`).

### Internal entry point
`registry::evaluate(id, input: &Value) -> Result<Evaluation>` where `Evaluation` holds the inner
`method`, `version`, `value: Value`, `explain` and `meta`. The macro gains a `dispatch_value_to`
arm next to `dispatch_to`, sharing the body of `execute` through a helper that takes an already
parsed input, so the envelope (including `input_hash`) is identical to `run` on the same JSON.
`run` keeps its `catch_unwind`; `evaluate` is only called from inside a `run`, so one guard
covers the whole analysis. A test asserts `evaluate(id, v)` equals `run(id, v.to_string())` for
every golden case of every non-analysis method.

### Pointers and values
- `serde_json::Value::pointer` and `pointer_mut` implement RFC 6901 (escapes `~0`, `~1`, no
  leading zeros, `-` not an index). They are reused; no new dependency.
- A leaf is a decimal when `serde_json::from_value::<Decimal>(leaf)` succeeds, the same
  deserialiser the inner methods use, so accepted spellings cannot diverge.
- `serde_json` is built with `arbitrary_precision`, so numbers in `base_input` keep their text
  and stay exact; overrides are written as `Value::String(decimal.to_string())`.

### Canonical `base_input` (R7)
`BaseInput` is a newtype over `serde_json::Map`. `preserve_order` is enabled in the workspace, so
its `Serialize` implementation sorts keys recursively to give the canonical form used by
`meta.input_hash`. Its `JsonSchema` implementation emits `{"type": "object"}` with a
description stating that the inner method validates it at run time (R8).

### Output types
Every output embeds `inner_method: InnerMethod`. Inner values travel as `serde_json::Value`
(goal seek `inner_result`, `solved_input`), and selected outputs as `Decimal`. A table cell is an
enum serialised untagged as `{"value": "..."}` or `{"error": {"code", "message"}}`, reusing
`ErrorReport`.

## Schema changes
Additive only: four new methods at version 1, `schemas/analysis.goal_seek/v1.json` and the
three others, four entries in `schemas/index.json`. No existing method version changes. An inner
method version bump never changes an analysis schema (R8). Optional fields with defaults:
`tolerance` (0.000000001) and `max_iterations` (100) in goal seek, documented in the schema.

## Numerical approach
- **Brent's method** on `Decimal` with checked arithmetic, following its common formulation:
  keep a bracketing pair with the best point `b`; try inverse quadratic interpolation when three
  distinct function values exist, otherwise the secant step; accept the step only if it stays
  within the usual bounds (inside the bracket, smaller than half the previous step, at least the
  minimum step 10^-24 × max(1, |b|)), otherwise bisect. Any checked operation that fails in an
  interpolation (division by a near-zero difference, overflow) falls back to bisection, so the
  path stays deterministic and never errors for arithmetic reasons inside the bracket.
- **Stopping rule:** |f(b)| ≤ tolerance, with f(x) = output(x) − target. The cap counts new
  evaluations after the two bracket ends. A discontinuous output never meets the rule at a jump,
  so it ends in `FM-E014` rather than a false root (AC4).
- **Deltas:** delta = scenario − base and swing = high − low are exact decimal subtractions;
  relative delta is one decimal division at the engine's documented precision.
- **No floats anywhere.** `argmin` is not used: its `ArgminFloat` bound requires
  `num_traits::Float`, which `Decimal` does not implement.

## Test strategy
| Test | Source of expected values | Proves |
| --- | --- | --- |
| Golden `analysis.goal_seek/irr-via-npv.yaml` | `tvm.irr` result and `scipy.optimize.brentq` (xtol 1e-15) | AC1, AC5 |
| Golden `analysis.goal_seek/dcf-implied-discount-rate.yaml` | `scripts/reference_values.py` (decimal, 50 digits, bisection) and `brentq` | AC2 |
| Golden `analysis.sensitivity/wacc-g-5x5.yaml` | reference script; numpy-financial recomputation; hand-built spreadsheet data table | AC6, AC7 |
| Golden `analysis.scenarios/dcf-three-cases.yaml` | reference script | AC9 |
| Golden `analysis.tornado/dcf-four-drivers.yaml`, `npv-tie.yaml` | reference script; tie by construction | AC10 |
| Unit tests `solver.rs` | closed-form roots (linear, quadratic), step function | AC3, AC4 |
| Unit tests `pointer.rs` | RFC 6901 examples and the cases of AC11 | AC11, AC15 |
| Contract cases `tests/contracts/analysis.*.yaml` | constructed | AC12, AC13, AC16, AC18, AC19 |
| Property tests (proptest, already planned in M1) | `registry::run` on the overridden input | AC8, AC14 |
| Integration test `tests/analysis_budget.rs` | wall-clock ceiling | AC17 |

The scipy and numpy-financial checks are cross-checks run by `scripts/cross_check_analysis.py`;
their outputs are recorded in the golden file comments. They are not CI dependencies unless the
maintainer adds them under `docs/en/project/dependencies.md`. The golden runner currently
compares flat fields; tables need nested expectations, so T2 extends it to accept JSON Pointer
keys in `expect` (additive, existing cases unaffected).

## Documentation
- Methodology pages in `docs/en/methods/` for the four methods, with formula, conventions, an
  example on `dcf.fcff` and a short "Verified against" section.
- A concept page `docs/en/concepts/analysis.md`: pointers, overrides as strings, caps, order,
  per-cell errors, why analysis does not nest.
- `docs/en/reference/errors.md`: five codes. `methods.md` regenerated.
- Python helpers in `python/fairmark/analysis.py` (translate only).
- Release note fragment in `changes/`; Italian edition through `/translate`.

## Risks and alternatives
- **Error code collision.** Other specs in progress may take `FM-E013` and above first. Codes
  are assigned at implementation in merge order; the spec lists them as provisional.
- **Cost of discarded inner explains.** Each inner evaluation builds its explain, then the
  analysis keeps only one number. Accepted in v1 for simplicity; an `explain: false` internal
  flag is a later optimisation behind the same contract.
- **Parallel evaluation rejected for v1.** Results are order-independent, so parallelism can be
  added later; the explain order and a dependency-free core weigh more today.
- **Typed composition rejected.** Generic analysis over typed inputs would need one analysis
  type per inner method; the registry already validates every input through its schema.
- **criterion rejected for v1.** A new dev-dependency for a non-gating number; the ignored
  release-mode test records the target instead.
- **Integer variables.** A pointer to `periods` passes the decimal check (it is a JSON number)
  and the inner method then rejects the string with `FM-E001`. Documented as out of scope.
