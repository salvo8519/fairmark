# SPEC-0008 — Analysis: goal seek, sensitivity tables, scenarios and tornado

| | |
| --- | --- |
| Status | Approved — 2026-11-07, maintainer decisions recorded below |
| Milestone | M2 (issue #45, WACC × g sensitivity grid); scenarios brought forward from M6 (part of issue #88) |
| Owner | @salvo8519 |
| Formula basis | Brent's root-finding method (bisection, secant, inverse quadratic interpolation) as commonly implemented; one- and two-way data tables, scenario analysis and tornado charts as practised in spreadsheet models; cross-checked against Brent's method written from its textbook formulation, bisection at 50 digits, `tvm.irr` and `pyxirr`, direct recomputation with numpy-financial and a hand-built spreadsheet data table |
| Related | ADR 0009 (themed methods, registry); ADR 0010 to be written (composition through the registry on JSON values, see Plan); SPEC-0002 (`tvm`), SPEC-0003 (`coc`), `dcf.fcff`; issues #45, #88 |

## Problem
A valuation is a point estimate built on assumptions the analyst is unsure of. Reviewers ask
the same questions every time: which discount rate gives this price (break-even), how the value
moves across a range of WACC and growth (data table), what the value is under a downside and an
upside case (scenarios), and which assumption matters most (tornado). Analysts answer them in
spreadsheets by hand, outside any audit trail. Fairmark can answer them deterministically by
running any registered method repeatedly, with every evaluation reproducible and every
convention reported.

## Scope
**In scope:** four methods in a new theme `analysis`, each generic over any registered method
that is not itself an analysis method: goal seek on one decimal input (`analysis.goal_seek`);
one-way and two-way sensitivity tables (`analysis.sensitivity`); named scenarios with deltas
against the base case (`analysis.scenarios`); tornado ranking of one output by swing
(`analysis.tornado`). Shared rules for JSON Pointers, evaluation caps, determinism and versioning.
**Out of scope:** Monte Carlo simulation (follow-up spec, see open question d); automatic bracket
expansion in goal seek (open question a); variables that are not decimals (integers such as
`periods`, enumerations, whole arrays); analysis of an analysis method; parallel evaluation;
pinning an inner method version in the input; charts (the output is data; rendering belongs to
adapters and reports).

## User stories
- As an analyst, I want the discount rate at which the DCF enterprise value equals an offer
  price, so that I can state the implied return of the offer.
- As an analyst, I want a WACC × g table of enterprise values in which invalid combinations
  (g ≥ WACC) are marked rather than aborting the table, so that I can paste it into a report.
- As an analyst, I want base, downside and upside scenarios with their deltas, so that the
  committee sees the range and its drivers in one result.
- As an AI agent, I want one generic contract for all four analyses, with typed errors for
  wrong pointers and caps, so that I can build requests from the method schema without guessing.

## Requirements (EARS)

### Common to all analysis methods
- **R1** — WHEN an analysis method runs, THE ENGINE SHALL evaluate the inner method named by
  `method` through the registry inside `fairmark-core`, at its current version, and SHALL record
  the inner method id and version in the result (`inner_method.id`, `inner_method.version`).
- **R2** — IF `method` names an analysis method, THEN THE ENGINE SHALL return `FM-E017`
  (nested analysis not supported); IF it names no registered method, THEN `FM-E007`.
- **R3** — THE ENGINE SHALL interpret every `variable`, override `pointer` and `output` as an
  RFC 6901 JSON Pointer. A variable or override pointer SHALL resolve, in `base_input`, to an
  existing member whose value is a decimal (a JSON number or a string that parses as a
  decimal); an output pointer SHALL resolve, in the `value` of the inner result, to a decimal.
  IF a pointer does not resolve, uses the `-` array token, or resolves to a non-decimal value,
  THEN THE ENGINE SHALL return `FM-E015` naming the pointer. Variable and override pointers are
  checked before any evaluation; output pointers on the first evaluation.
- **R4** — WHEN the engine sets a variable, THE ENGINE SHALL write the decimal as a JSON string
  at the pointer and leave every other member of `base_input` unchanged; the inner method
  validates each resulting input and its errors keep their own codes.
- **R5** — THE ENGINE SHALL bound the number of inner evaluations of one request by 10,000.
  IF the evaluations a request needs (for goal seek: 2 + `max_iterations`) exceed this cap,
  THEN THE ENGINE SHALL return `FM-E016` before the first evaluation. `max_iterations` above
  1,000 or below 1 is `FM-E001`.
- **R6** — THE ENGINE SHALL evaluate sequentially in a fixed, documented order (bracket ends
  then iterations; table rows then columns; scenarios and tornado variables in declaration
  order, low before high), so that the same input and versions produce the same output bit for
  bit, including the order of `explain.steps`.
- **R7** — THE ENGINE SHALL compute `meta.input_hash` over the whole analysis input, with
  `base_input` in canonical form (object keys sorted recursively), so that requests differing
  only in key order hash identically.
- **R8** — THE ENGINE SHALL publish the input schema of each analysis method with `base_input`
  as a free JSON object; the analysis schema SHALL NOT depend on the schema of any inner method.
- **R9** — WHEN running `analysis.sensitivity`, `analysis.scenarios` or `analysis.tornado`,
  THE ENGINE SHALL first evaluate `base_input` unchanged and report its outputs as the base
  case; IF that evaluation fails, THEN THE ENGINE SHALL return its error unchanged.

### Goal seek
- **R10** — WHEN given `method`, `base_input`, `variable`, `output`, `target` and
  `bracket` [low, high], THE ENGINE SHALL find x in [low, high] such that
  |output(x) − target| ≤ `tolerance` by Brent's method, and return the solution, the achieved
  output, the residual (achieved − target), the iterations, the solved input and the full
  inner result at the solution. IF an end of the bracket already satisfies the tolerance,
  THEN THE ENGINE SHALL return it with zero iterations (low first).
- **R11** — IF low ≥ high or `tolerance` ≤ 0, THEN THE ENGINE SHALL return `FM-E001`; IF
  output(low) − target and output(high) − target have the same strict sign and neither end
  satisfies the tolerance, THEN THE ENGINE SHALL return `FM-E013` (target not bracketed)
  reporting both values.
- **R12** — IF the tolerance is not met within `max_iterations`, THEN THE ENGINE SHALL return
  `FM-E014` (goal seek did not converge) reporting the best x and its residual.
- **R13** — THE ENGINE SHALL report one explain step per evaluation (bracket low, bracket high,
  then each iteration with its step type: bisection, secant or inverse quadratic
  interpolation, and the output and residual in the formula), and in `explain.conventions` the
  algorithm, the tolerance and its units, the iteration cap, the bracket and the minimum step.
- **R14** — IF an inner evaluation fails during goal seek, THEN THE ENGINE SHALL return that
  error unchanged (a root search over a partly undefined function is not defined).

### Sensitivity
- **R15** — WHEN given `rows` (a variable and a list of values) and optionally `columns`
  (another variable and values), THE ENGINE SHALL evaluate the inner method once per cell with
  both variables set, in row-major order, and return the selected `output` per cell, the axes
  and the base case. Row and column variables SHALL differ (`FM-E001` otherwise).
- **R16** — IF the evaluation of a cell fails with any code other than `FM-E012`, THEN THE
  ENGINE SHALL record that code and message in the cell and continue; `FM-E012` (a defect)
  SHALL abort the request. The number of failed cells SHALL be reported.
- **R17** — THE ENGINE SHALL report one explain step per successful cell (variable values in the
  label, output value as the step value) and in `explain.conventions` the inner method and
  version, the output pointer, the evaluation order and the per-cell error rule.

### Scenarios
- **R18** — WHEN given named scenarios, each a list of overrides (pointer, decimal value), and a
  list of `outputs`, THE ENGINE SHALL evaluate each scenario on `base_input` with its overrides
  applied and return, per scenario and per output, the value, the delta (scenario − base) and
  the relative delta (delta / base, `null` when the base output is zero). Scenario names SHALL
  be unique and non-empty; pointers within one scenario SHALL be unique (`FM-E001` otherwise).
- **R19** — IF a scenario fails, THEN THE ENGINE SHALL record the error in that scenario under
  the rule of R16 and continue.

### Tornado
- **R20** — WHEN given a list of variables, each with `low` < `high`, and one `output`, THE
  ENGINE SHALL evaluate the output with each variable set to its low and to its high value, the
  others at base, and return per variable the two outputs, the swing (output at high − output
  at low) and its absolute value, ranked by absolute swing in descending order, ties kept in
  declaration order.
- **R21** — IF variables repeat or `low` ≥ `high`, THEN THE ENGINE SHALL return `FM-E001`; IF
  any evaluation fails, THEN THE ENGINE SHALL return its error unchanged (a ranking with
  missing swings is not defined).

### Budget and errors
- **R22** — THE ENGINE SHALL complete a 21 × 21 sensitivity table on `dcf.fcff` with ten
  forecast years within the budget stated in the acceptance criteria.
- **R23** — THE ENGINE SHALL document the new error codes in `docs/en/reference/errors.md`.

## Conventions and parameters
| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Root-finding algorithm | Brent's method | fixed in v1 | common practice; textbook formulation (Numerical Recipes `zbrent`), as in `scipy.optimize.brentq` |
| Bracket | explicit [low, high] | required | explicit and auditable; no automatic search in v1 |
| Tolerance | absolute, on the output, in output units | 0.000000001 | stated on the output so it reads in the analyst's units (currency, rate); documented in the schema |
| Iteration cap | 1 to 1,000 | 100 | fixed bound, reported |
| Minimum step on the variable | 10^-24 × max(1, \|x\|) | fixed in v1 | keeps Brent's step above decimal resolution; reported |
| Evaluation cap per request | 10,000 | fixed | predictable cost |
| Failed table cells and scenarios | recorded per cell (R16) | recorded | spreadsheet data tables show an error per cell; open question b |
| Tornado shocks | explicit low and high | required | open question c |
| Relative delta when the base output is zero | `null` | fixed | no silent defaults |
| Override encoding | decimal written as a JSON string | fixed | keeps exactness |

## Acceptance criteria
- **AC1** (R10) — Goal seek on `tvm.npv` (`/rate`, output `/npv`, target 0, bracket [0, 1],
  tolerance 1e-18) for flows [-1000, 300, 400, 500] matches `tvm.irr` within 1e-15 and
  `pyxirr.irr` within a relative 1e-9; the engine reproduces the iteration count and solution
  (within 1e-24) of Brent's method written independently from its textbook formulation in
  `scripts/reference_analysis.py`. (Amended at implementation: `pyxirr` is already a test
  oracle; `scipy` would be a new, heavy dependency for the same evidence.)
- **AC2** (R10) — Goal seek on `dcf.fcff` for `/discount_rate` such that `/enterprise_value`
  equals a target matches the reference script (Python `decimal`, 50 digits, bisection to
  1e-30) within 1e-12 and reproduces the textbook Brent path; the residual is at most the
  tolerance; `solved_input` at `/discount_rate` is a JSON string.
- **AC3** (R11) — A bracket that does not contain the target returns `FM-E013`; low ≥ high and
  tolerance 0 return `FM-E001`; a bracket end that meets the target returns it with 0
  iterations.
- **AC4** (R12) — The case of AC2 with `max_iterations` 1 returns `FM-E014`; a step-function
  output (discontinuous at the target) returns `FM-E014`, never a false solution.
- **AC5** (R13) — The explain of AC1 has 2 + iterations evaluation steps in order, then one
  solution step; each iteration is labelled with one of the three step types; the conventions name the algorithm,
  tolerance, cap, bracket and minimum step.
- **AC6** (R15, R17) — A 5 × 5 WACC × g table on `dcf.fcff` matches the reference script within
  1e-9 currency units, numpy-financial recomputation within a relative 1e-9, and a hand-built
  spreadsheet data table (committed as reference values with its construction described in the
  golden file) at the precision recorded there.
- **AC7** (R16) — In the table of AC6, cells with g ≥ WACC carry `FM-E004` and every other cell
  equals its value in a table without those cells; the failed-cell count is correct; an
  injected `FM-E012` aborts the request (unit test).
- **AC8** (R4, R15) — Property: every cell equals `registry::run` on the inner method with the
  overridden input; a one-way table equals the corresponding column of a two-way table.
- **AC9** (R18, R19) — Three scenarios on `dcf.fcff` match the reference script; deltas equal
  scenario − base exactly; relative delta is `null` where the base output is zero; a scenario
  with an invalid override records its error and the others are returned.
- **AC10** (R20) — A tornado on `dcf.fcff` with four variables matches the reference ranking and
  swings; on `tvm.npv` at rate 0 two cash flows shocked by ±1 tie and keep declaration order.
- **AC11** (R3) — Each returns `FM-E015`: missing member, array index out of range, `-` token,
  pointer without leading `/`, variable pointing to a non-decimal (`/timing`), output pointing
  to an array (`/present_values`).
- **AC12** (R2) — `method: "analysis.sensitivity"` returns `FM-E017`; `method: "x.y"` returns
  `FM-E007`.
- **AC13** (R5) — A 101 × 100 table returns `FM-E016` without evaluating the inner method;
  `max_iterations` 1001 returns `FM-E001`.
- **AC14** (R6, R7) — Running each golden request twice yields byte-identical JSON; permuting
  the keys of `base_input` leaves `meta.input_hash` and `value` unchanged.
- **AC15** (R4) — Base inputs giving a value as the number `0.1` or the string `"0.1"` produce
  identical analysis values.
- **AC16** (R8) — `schemas/analysis.*/v1.json` describe `base_input` as an object with no inner
  schema; contract cases in `tests/contracts/analysis.*.yaml` pass.
- **AC17** (R22) — A test runs the 21 × 21 table of R22 in the test profile on CI and asserts
  442 evaluations and a wall time below 10 seconds; an ignored release-mode test records the
  target of 250 milliseconds on the reference CI runner (not gating, so the gate cannot flake).
- **AC18** (R9) — A base input that the inner method rejects returns the inner error code.
- **AC19** (R14) — A bracket end at which the inner method fails returns that inner code.
- **AC20** (R1) — Every analysis result reports `inner_method` equal to the registry catalogue
  entry of the inner method.
- **AC21** (R23) — `docs/en/reference/errors.md` lists the new codes; `scripts/docs_sync.py`
  passes.

## Interfaces
- Method ids: `analysis.goal_seek`, `analysis.sensitivity`, `analysis.scenarios`,
  `analysis.tornado`, all version 1.
- Input schema changes: additive (four new methods; no existing schema changes).
- Common input fields: `method` (id), `base_input` (object). Goal seek: `variable`, `output`,
  `target`, `bracket` [low, high], `tolerance`, `max_iterations`. Sensitivity: `output`,
  `rows {variable, values}`, `columns {variable, values}` (optional). Scenarios: `outputs`,
  `scenarios [{name, overrides [{pointer, value}]}]`. Tornado: `output`,
  `variables [{variable, low, high}]`.
- Output fields: all carry `inner_method {id, version}`. Goal seek: `solution`, `achieved`,
  `residual`, `iterations`, `tolerance`, `solved_input`, `inner_value` (the `value` of the inner
  result; running the inner method on `solved_input` reproduces it with its full explain). Sensitivity: `base`,
  `rows`, `columns`, `cells` (each `{value}` or `{error {code, message}}`), `failed_cells`.
  Scenarios: `base`, `scenarios [{name, outputs [{output, value, delta, relative_delta}] |
  error}]`. Tornado: `base`, `ranking [{variable, low, high, output_low, output_high, swing,
  abs_swing}]`.
- New error codes (provisional, next free at implementation): `FM-E013` target not bracketed,
  `FM-E014` goal seek did not converge, `FM-E015` invalid analysis pointer, `FM-E016`
  evaluation cap exceeded, `FM-E017` nested analysis not supported. `FM-E005` is not reused:
  its documented meaning is the IRR and XIRR solver, and codes never change meaning. `FM-E001`
  is not reused for pointers: the inner method already reports invalid inputs as `FM-E001`, and
  an agent must be able to tell a wrong pointer from a wrong base input.
- Schema versioning: an inner method version bump does not change any analysis schema; the
  analysis result records the inner version actually run, and reproduction needs the engine
  version in `meta`.

## Decisions
The maintainer accepted every recommendation on 2026-11-07: explicit bracket, per-cell
errors, explicit tornado shocks, Monte Carlo in a follow-up spec, and composition through the
registry for the analysis theme (ADR 0010). Results must never be ambiguous: agents combine and
interpret them, the engine guarantees that each one is exact, validated and verified.

### Questions as asked
- [x] **a. Automatic bracket expansion in goal seek v1?** Recommended: no. An explicit bracket is
  auditable and fails loudly with `FM-E013`; expansion adds a second convention (growth factor,
  limits) for little gain. Revisit with user feedback; it can be added as an optional field.
- [x] **b. Per-cell errors or fail-fast in tables and scenarios?** Recommended: per-cell (R16).
  A WACC × g table naturally contains g ≥ WACC cells, and spreadsheet data tables show an error
  per cell. Defects (`FM-E012`) still abort. An `on_error: "fail"` option can be added later.
- [x] **c. Symmetric percentage shocks in tornado as a convenience?** Recommended: explicit low
  and high only. "±10% on a 9% rate" is ambiguous (8.1–9.9% or 8–10%); explicit values remove
  the ambiguity and an adapter can still offer a helper that writes them.
- [x] **d. Monte Carlo with a seeded generator in a follow-up spec?** Recommended: yes, a
  separate spec with a named generator (PCG64), the seed and the generator name in the input
  and in `explain.conventions`, and distributions sampled in decimal arithmetic.
