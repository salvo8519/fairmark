# SPEC-0001 — Plan

Status: active (SPEC-0001 approved on 2026-11-08). This plan covers the shared DCF engine and
`dcf.fcff` version 3 (issue #110). The other issues of the spec add to it through the extension
points listed at the end; each extends this plan in its own pull request.

## Design
Module layout in `crates/fairmark-core`:

| File | Content |
| --- | --- |
| `src/dcf.rs` | Theme root: module declarations and re-exports only, so that parallel issues add one line each |
| `src/dcf/engine.rs` | The shared engine, independent of the basis: stages and projection, discount rate schedule, terms check, timing, discount factors, terminal value hookup, period table, explain |
| `src/dcf/fcff.rs` | `dcf.fcff` version 3: input and output types, the mapping of `wacc` fields to the engine, the bridge from value of operations to equity value on the enterprise basis, the `Method` registration |

The engine is called by typed Rust calls (ADR 0009), never through the registry. It takes an
internal `EngineInput` that borrows the stages, the base flow, the terms, a `RateInput`
(one rate, or one rate per period plus a terminal rate), the timing, the terminal discounting
time, the terminal technique and the basis, together with the JSON names of the rate fields of
the calling method (`wacc`, `wacc_per_period`, `terminal_wacc` for `dcf.fcff`), so that every
error names the field of the method the caller used (R27). It returns an `EngineOutput`: the
period table, the present value of the explicit flows, the terminal block and the explain
(steps and conventions). The bridge is not in the engine: each basis has its own (R23, R24).

Building blocks reused, not duplicated:
- `tvm::period_discount_factors` for every discount factor of the explicit periods, with one
  rate or one rate per period and end or mid-period timing (R12, SPEC-0002 R5); the factor at
  the horizon is the end-of-period factor of period N from the same function.
- `tv::gordon` (typed call) for the terminal value at N (R19, R20); its explain steps and
  conventions are copied into the DCF explain with the prefix "terminal value (tv.gordon v1)".
- `ratios::NetFinancialPosition` of SPEC-0007 and `tv::Line` for labelled amounts (R23, R25).
- `types::Timing` for `end_of_period` and `mid_period`.

The domain types announced by ADR 0009 point 5 (`Rate`, `CashFlowSeries`) are not introduced
by this issue: the engine keeps internal types only (`ResolvedStage` with `StageKind`,
`Projected`, `RateInput`, `RateFields`, `EngineInput`, `EngineOutput`) and JSON inputs keep
plain decimals. Whether they become the ADR types is decided with #167, when a dated time axis
gives `CashFlowSeries` its second form.

## Schema changes
`dcf.fcff` version 2 to version 3, **breaking** (approved, spec "Recorded approval of the
numerical change"). Version 3 input (all choices required, no defaults, R29):

| Field | Type | Rule |
| --- | --- | --- |
| `stages` | list of 1 to 3 objects `{periods, projection, flows?, growth?, from_growth?, to_growth?}` | `projection` is `given`, `constant_growth` or `linear_decline`; each projection takes only its own fields |
| `base_flow` | decimal, optional | required when the first stage is generated, refused when it is `given` (`FM-E025`) |
| `cash_flow_terms`, `discount_rate_terms` | `nominal` or `real`, required | different terms are refused (`FM-E024`) until expected inflation is accepted (#166) |
| `wacc` or `wacc_per_period` + `terminal_wacc` | decimals | exactly one form; `terminal_wacc` only with `wacc_per_period` (R12, Decision 13) |
| `timing` | `end_of_period` or `mid_period`, required | no default (was `end_of_period` in version 2) |
| `terminal_discount_time` | `horizon` or `mid_period` | required under `mid_period`, refused under `end_of_period` (R21, Decision 6) |
| `terminal` | object with exactly one key, the `tv.*` method id | `{"tv.gordon": {growth, first_flow, normalised_next_flow?, growth_cap?}}` |
| `net_financial_position` | SPEC-0007 structure, optional | none given means none, reported |
| `non_operating_assets` | list of `{label, value}`, optional | values zero or more |

Choices with several options are flat objects with a discriminating enumeration plus optional
fields (stages, terminal), not tagged unions. Reason: `scripts/schema_snapshots.py` treats a
new value of an enumeration and a new optional field as additive, but a new member of a
`oneOf` of objects as breaking. This shape keeps R29 ("a later option is an additive change")
true under the checker that CI applies: a new projection kind, terminal technique or rate form
adds an enumeration value or an optional field to version 3. The terminal object is written
exactly like an entry of `tv.compare` (`{"tv.gordon": {...}}`), with every technique an
optional property and exactly one required at run time. The number of stages (at most three)
is checked at run time, not with `maxItems`, so that more stages remain an additive change.

Output of version 3: `basis`, `value_of_operations`, `enterprise_value`, `equity_value`,
`pv_explicit_period`, `terminal_value_weight` (`null` when the value of operations is zero),
`periods` (R10), `terminal` (technique, version, rate, value at N, discounting time, factor,
present value, the technique's own output), `bridge` (each line echoed).

## Numerical approach
In implementation order, all in `Decimal` with checked arithmetic (`crate::num`):
1. Horizon N = Σ periods (at most 10,000, `FM-E001`), stage and rate counts checked (`FM-E025`).
2. Projection, period by period: `given` copies the flow; `constant_growth` multiplies the
   previous flow by (1 + g); `linear_decline` uses g_k = g_from + ((g_to − g_from) × k) / m,
   multiplying before dividing so that the only rounding is the final division (Decision 4).
3. Rates: one rate repeated N times or the per-period list; every rate greater than −1
   (`FM-E003` naming the period).
4. Discount factors from `tvm::period_discount_factors`, reported in the period table: end
   of period DF_k = DF_{k−1} / (1 + r_k); mid period DF_{k−1} / (1 + r_k)^0.5 (square root of
   `rust_decimal`). Present values are computed in `run` by dividing each flow by its
   compounding factor G_k = G_{k−1} × (1 + r_k) (mid period: G_{k−1} × (1 + r_k)^0.5), so that
   they keep every significant digit at long horizons (numerical audit, finding 1); when G_k
   leaves the decimal range the engine multiplies by DF_k and reports it.
5. Terminal value through `tv::gordon` at the terminal rate (the single rate, or
   `terminal_wacc`), with CF_{N+1} = CF_N × (1 + g) or the given normalised flow.
6. Terminal discounting: `horizon` uses DF_N; `mid_period` uses DF_N × (1 + r_N)^0.5 with
   the rate of period N (R21); the present value is TV_N / G_N, or TV_N × (1 + r_N)^0.5 / G_N.
7. Value of operations = Σ PV + PV(TV); enterprise value = value of operations + Σ
   non-operating assets; equity value = enterprise value − net financial position.
8. Terminal value weight = PV(TV) / value of operations.

Version 2 used `tvm::discount_factor` (direct powers) and multiplied by it; version 3 divides
by the compounding factor. Against the 50-digit reference the value of operations agrees to
about 1e-24 relative or better, also over 300 to 1,000 periods: the migrated version 2 golden
cases keep their expected values at their tolerance of 1e-9 (R30, AC15).

## Test strategy
- Independent reference `scripts/reference_dcf.py` (Python `decimal`, 50 digits in a local
  context, written from the spec, sharing no code with the engine: discount factors as direct
  powers of the cumulated growth factor, mid-period terminal by its formula).
- Golden cases `tests/golden/dcf.fcff/`: three-stage (given, linear decline, Gordon) and
  two-stage from a base flow with rates per period (AC4); mid-period terminal at N and N − 0.5
  (AC12); bridge with the net financial position and labelled non-operating assets
  (R23 for the items of this issue); Damodaran Illustration 15.3 (Amgen) at the published
  precision and at the reference precision (AC4, Decision 12); every error of this issue
  (AC6, AC7, AC8, AC12); the version 2 cases re-expressed (AC15).
- Python `python/tests/test_reference_dcf.py`: grids against the reference, properties AC5
  (constant growth equals given flows; linear decline with equal growths equals constant
  growth) and AC7 (factors equal those of `tvm.present_value`, exactly).
- Cross-check (agreement of conventions only, binary floating point): numpy-financial NPV of
  the explicit flows plus the Gordon value, in `python/tests/test_cross_check.py`.
- Rust unit tests: errors and their JSON pointers (AC14, R27), warnings (AC10 for the
  warnings of this issue), every convention reported (AC14), no boolean in the schema (AC14).
- Contract cases `tests/contracts/dcf.fcff.yaml` re-expressed in version 3.

## Documentation
Methodology page `docs/en/methods/dcf-fcff.md` rewritten for version 3 and its Italian
edition; examples using `dcf.fcff` in guides, concepts and analysis pages re-expressed;
reference regenerated (`uv run just generate`); release notes `changes/110.breaking.md` and
`changes/110.feature.md`; status stays preview.

## Extension points (other issues of this spec)
Each is additive to version 3 under the schema checker.
- **#38 `dcf.free_cash_flows`**: a new module `src/dcf/free_cash_flows.rs` (one line in
  `src/dcf.rs`); a `given` stage accepts build-up lines through a new optional field `lines`
  next to `flows` (exactly one of the two), resolved by `check_stages` into a new `StageKind`
  variant that becomes amounts before projection.
- **#165 `dcf.fcfe`**: a new module `src/dcf/fcfe.rs` that maps `cost_of_equity`,
  `cost_of_equity_per_period`, `terminal_cost_of_equity` to the engine's `RateInput` with
  its own `RateFields`, passes `TerminalBasis::Equity`, and implements the equity bridge (R24).
  The engine already carries the basis and reports it.
- **#166 nominal and real terms**: optional fields `inflation`, `inflation_per_period` and
  `terminal_inflation`; the engine's `check_terms` step, which today refuses different terms
  with `FM-E024`, converts the rate schedule through `rates::real_nominal` instead and fills
  `rate_used` of the period table, which the output already reports next to `discount_rate`
  (R16 to R18).
- **#43 terminal technique and discounting time**: new optional properties of the `terminal`
  object (`tv.h_model`, `tv.exit_multiple`, `tv.value_driver`, `tv.limited_horizon`,
  `tv.normalised_income`, `tv.liquidation`), validated in `check_terminal` and computed by one
  arm each in `terminal_value`, with the basis check (`FM-E019`) and the warning for a price discounted at
  N − 0.5. The terminal discounting time of Decision 6 is already in version 3.
- **#44 bridge**: optional `debt_like_items` and `non_controlling_interests` (labelled amounts),
  added to `bridge` in `dcf/fcff.rs` and reported (the negative equity warning of R26 is
  already in place).
- **#167 dated periods**: optional `valuation_date` and period end dates; the engine's time
  axis (today t = k or k − 0.5) becomes the dated axis of `tvm::time_axis`.

## Risks and alternatives
- **Tagged unions** (`{"projection": {"linear_decline": {...}}}`) would give a tighter JSON
  Schema, but every new option would be a breaking change for the CI checker and force a new
  method version, against R29. Rejected; run-time checks name the field and the fix instead.
- **Terminal technique scope**: only `tv.gordon` is wired in this issue, which is the minimum
  for version 3 to produce a value; the other techniques are additive (#43).
- **Terms without inflation**: the two required terms are in version 3 now, so that #166 adds
  only optional fields; a mismatch is refused with `FM-E024` (R15) until then.
- **Analysis golden cases** use `dcf.fcff` as their inner method with version 2 inputs; they
  are re-expressed in version 3 with identical expected values in the migration commit and
  need the maintainer's review with the `numerical-change` label.

## Open questions for the maintainer
- **Implied growth of a given last stage (R11).** The warning for a growth discontinuity is
  reported only when the last explicit period is generated, because only then is a growth
  applied. For a `given` last stage the growth implied by the last two flows,
  CF_N / CF_{N−1} − 1, could be compared with the perpetual growth too. Not implemented: the
  spec does not say whether an implied growth counts as "the growth of the last explicit
  period".
- **Analysis golden cases** re-expressed in version 3 (see "Risks and alternatives"): awaiting
  explicit approval.
- **Tolerances of the Amgen published-precision cases** (2, and 20 for the terminal value):
  awaiting acceptance; the golden files decompose the differences.
