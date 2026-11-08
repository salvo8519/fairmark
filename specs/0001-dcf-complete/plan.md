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

## Issue #43 — Terminal value technique and discounting time
Extends version 3 additively through the extension point above.

| Item | Design |
| --- | --- |
| Module | `src/dcf/terminal.rs` holds the terminal input types (moved from `engine.rs`, same schema), the checks and the computation; `engine.rs` calls it at the two places it called the Gordon code, so that the parallel issue #38 (stages) and this one touch disjoint parts of the engine |
| Techniques | Optional properties of `terminal`: `tv.gordon` (#110), `tv.h_model`, `tv.value_driver`, `tv.exit_multiple`, `tv.limited_horizon`, `tv.normalised_income`, `tv.liquidation`; exactly one present, checked at run time (`FM-E001` naming the keys) |
| Rate | The terminal rate (the single rate, or `terminal_wacc`) is passed to every technique that takes a rate (R19, Decision 13), including the capitalisation rate of `tv.normalised_income`, which accepts no separate rate inside the DCF (the SPEC-0006 decision on its own capitalisation rate concerns `tv.compare` only); `tv.exit_multiple` uses it only for the implied growth when `next_flow` is given; `tv.liquidation` takes none |
| Terms | The technique's own terms (SPEC-0006), the rate and the last explicit flow excluded; a growth cap where the technique accepts one (R22). Terms that the standalone method defaults (`growth` and `residual_value` of `tv.limited_horizon`, `growth` of `tv.normalised_income`) are required inside the DCF (R29) |
| First terminal flow | `tv.gordon`: `last_flow_grown` or `normalised_next_flow` (#110). `tv.h_model`: `first_flow` required; `last_flow_grown` passes CF_N to the technique; `normalised_next_flow` is refused (`FM-E001`) until the maintainer decides how a flow at N + 1 enters a technique defined on CF_N (open question below) |
| Basis (R19) | `tv.exit_multiple`: the basis of the metric; `tv.normalised_income`, `tv.liquidation`: the declared basis; each must equal the basis of the DCF, otherwise `FM-E019` naming the field and the fix. The flow-based techniques take the basis of the flows they are given |
| Discounting (R21, Decision 6) | Unchanged: DF_N, or DF_N × (1 + r_N)^0.5 at N − 0.5. Warnings: `tv.exit_multiple` or `tv.liquidation` discounted at N − 0.5 (a price at a date); a perpetual-growth technique (`tv.gordon`, `tv.h_model`, `tv.value_driver`, `tv.normalised_income`) discounted at N under mid-period timing |
| R11 | The growth of the last generated period is compared with the perpetual growth stated by the technique: `growth` (`tv.gordon`, `tv.value_driver`, `tv.normalised_income`), `long_term_growth` (`tv.h_model`); the growth implied by an exit multiple is reported, not compared, and a convention says so |
| Errors (R27) | The DCF checks only what it alone knows (the number of techniques, the first terminal flow, the basis); the terms are checked by the technique's own `check` function, before any calculation, which reports the field of each error (`tv::FieldError`); the DCF turns it into `/terminal/<id>/<field>`, or the terminal rate field for the rate, with the code unchanged (`FM-E004` with its pointers). The standalone methods keep their messages |
| Output | `terminal.technique`, `terminal.technique_version`, the technique's own output in `terminal.output` (untagged, as in `tv.compare`) |

Tests: `scripts/reference_dcf.py` restates every technique (limited horizon by explicit
summation); golden cases `terminal-*.yaml` and `error-terminal-*.yaml`; Python property AC11
against each `tv.*` method run alone through the registry.

## Risks and alternatives
- **Tagged unions** (`{"projection": {"linear_decline": {...}}}`) would give a tighter JSON
  Schema, but every new option would be a breaking change for the CI checker and force a new
  method version, against R29. Rejected; run-time checks name the field and the fix instead.
- **Terminal technique scope**: only `tv.gordon` is wired in this issue, which is the minimum
  for version 3 to produce a value; the other techniques are additive (#43).
- **Terms without inflation**: the two required terms are in version 3 now, so that #166 adds
  only optional fields; a mismatch is refused with `FM-E024` (R15) until then.
- **Analysis golden cases** use `dcf.fcff` as their inner method with version 2 inputs; they
  are re-expressed in version 3 with identical expected values in the migration commit;
  approved by the maintainer on 2026-11-08 (spec, "Recorded approval of the numerical change").

## Open questions for the maintainer
- **Implied growth of a given last stage (R11).** The warning for a growth discontinuity is
  reported only when the last explicit period is generated, because only then is a growth
  applied. For a `given` last stage the growth implied by the last two flows,
  CF_N / CF_{N−1} − 1, could be compared with the perpetual growth too. Not implemented: the
  spec does not say whether an implied growth counts as "the growth of the last explicit
  period".
- **First terminal flow of `tv.h_model` (#43, R20).** `tv.h_model` is defined on CF_N. A
  normalised flow at N + 1 would have to be brought back to N, at g_s (the first growth of the
  decline) or at g_l; the spec does not say which. Refused until decided.
- **R11 with `tv.h_model` (#43).** The literal comparison is with the perpetual growth g_l; the
  growth right after N in the H-model is g_s. Implemented literally (g_l); g_s may be the
  comparison the maintainer intends.
- **Mid-period warning for `tv.limited_horizon` (#43, Decision 6).** Decision 6 names the exit
  multiple and the liquidation (warning at N − 0.5); the existing warning at N concerns
  perpetual-growth values. A limited horizon is a finite set of flows plus a residual value (a
  price at N + n): no warning is reported either way until decided.

## Issue #38: free cash flow build-up (`dcf.free_cash_flows`)

Requirements: R1 (the build-up method id and version), R2, R3, R4, R5, R27 (validation errors
of the build-up; deserialisation errors of the shared registry carry no pointer yet), R28 and
R29 for the fields of the build-up; acceptance criteria AC1, AC2, AC3 and the build-up part of AC14.

### Module layout in `crates/fairmark-core`
| File | Content |
| --- | --- |
| `src/dcf/free_cash_flows.rs` (new) | `FreeCashFlowsInput`, `FreeCashFlowLines`, `Adjustment`, `NegativeEbit`, `TaxForm`, `FreeCashFlowsOutput`, `FreeCashFlowPeriod`; `pub fn free_cash_flows`; unit type `FreeCashFlows` implementing `Method` (id `dcf.free_cash_flows`, version 1); unit tests |
| `src/dcf.rs` | `pub mod free_cash_flows;` and the re-export of the public items, in the module list of #110 |
| `src/dcf/engine.rs`, `src/dcf/fcff.rs` | The extension point of #110 for #38: `lines` on a `given` stage and the tax fields of `dcf.fcff` (see "Lines on `given` stages") |
| `src/registry.rs` | One line, `dcf::FreeCashFlows`, in the `dcf` group |
| `src/error.rs` | Unchanged: every refusal of the build-up is `FM-E001` (R5, R27); no new code |

The type path `fairmark_core::dcf::free_cash_flows::FreeCashFlows` satisfies the ADR 0009 test
(the path starts with the theme module `dcf`). The build-up lives in its own file so that the
shared engine of #110 can be written in `dcf.rs` (or a `dcf/` folder) without conflicts.

### Input
One list `periods` of per-period line items, in currency units per period, plus the tax
choice, which applies to every period:

| Field | Meaning | Rule |
| --- | --- | --- |
| `periods[].ebit` | EBIT of the period | any sign |
| `periods[].operating_taxes` | Taxes on EBIT given as an amount (amounts form) | any sign; positive is a tax paid |
| `periods[].depreciation_and_amortisation` | D&A added back | zero or more (R5) |
| `periods[].capital_expenditure` | Capex subtracted | zero or more (R5) |
| `periods[].change_in_net_working_capital` | ΔNWC subtracted | signed: positive for an increase, which absorbs cash (R5) |
| `periods[].other_adjustments` | Labelled adjustments added | signed: positive for an inflow (R5); default none |
| `periods[].interest_expense` | Interest for FCFE | zero or more (R5); for every period or for none |
| `periods[].net_borrowing` | Net borrowing for FCFE | signed: positive for new borrowing; with `interest_expense` |
| `notional_tax_rate` | Tax rate on EBIT (rate form) | 0 ≤ t < 1, as `coc.*` |
| `negative_ebit` | `credit` or `zero` | required with the rate form, refused with the amounts form (R4, Decision 8) |
| `interest_tax_rate` | Rate t of the interest tax shield | required with the amounts form when financing lines are given; refused otherwise (R3) |

Exactly one tax form (Decision 7): `notional_tax_rate` with no `operating_taxes` in any period,
or `operating_taxes` in every period with no `notional_tax_rate`. Any other combination is
`FM-E001` naming the fields by JSON pointer (`/notional_tax_rate`,
`/periods/2/operating_taxes`) and stating the fix (R27). The form is chosen by which fields are
present, as the rate forms of `loans.*` and the flow choice of `tv.gordon`; no boolean (R29).
`serde(flatten)` is not used because it does not combine with `deny_unknown_fields`.

### Computation (per period t)
- Rate form: taxes_t = t × EBIT_t; when EBIT_t < 0, `credit` keeps t × EBIT_t (a saving) and
  `zero` sets taxes_t = 0. No tax loss carry-forward (R4). Amounts form: taxes_t as given.
- FCFF_t = EBIT_t − taxes_t + D&A_t − capex_t − ΔNWC_t + Σ adjustments_t (R2).
- FCFE_t = FCFF_t − interest_t × (1 − t) + net borrowing_t, with t = `notional_tax_rate` in
  the rate form and `interest_tax_rate` in the amounts form (R3). In a period where the `zero`
  treatment applies, t = 0: no interest tax shield, the interest is subtracted in full and
  the FCFE equals the net-income route (amendment 14 of the spec, approved by the maintainer
  on 2026-11-08). The rate applied in each period is in the output (`interest_tax_rate`) and in
  `explain.conventions`.
- All arithmetic through `crate::num` (checked; overflow is `FM-E008`).

### Output and reuse by the shared engine (#110, #165)
`FreeCashFlowsOutput` holds `periods` (one `FreeCashFlowPeriod` per period with every
component, the FCFF and, when financing lines are given, the after-tax interest, the net
borrowing and the FCFE), the two flow lists `free_cash_flows_to_firm` and
`free_cash_flows_to_equity` (`null` without financing lines), the tax form, the negative EBIT
treatment and the rate of the interest tax shield.

The shared engine reuses the build-up for a `given` stage with build-up lines (R7): the stage
carries `Vec<FreeCashFlowLines>`, the DCF input carries the three tax fields, and the engine
calls `dcf::free_cash_flows::build_up(lines, &TaxChoice { .. }, Location { pointer_prefix,
first_period }, &mut explain)` (a typed call, ADR 0009). `Location` gives the JSON pointer of
the list of lines (for example `/stages/0/lines`) and the period number of its first line, so
that error pointers, "period n" texts and explain labels refer to the engine's input; the steps
and conventions are appended to the engine's explain. The engine takes
`free_cash_flows_to_firm` for `dcf.fcff` or `free_cash_flows_to_equity` for `dcf.fcfe`.
`free_cash_flows()` is `build_up` with `("/periods", 1)`. The pointers of the three tax fields
are at the root of the input (`/notional_tax_rate`, `/negative_ebit`, `/interest_tax_rate`). The types are public and
re-exported from `dcf`; the count check of R7 (`FM-E025`) stays in the engine.

### Lines on `given` stages (R7, extension point of #110)
R7: "WHEN a stage's projection is `given`, THE ENGINE SHALL take its flows either as amounts or
as build-up lines (R2, R3); IF the number of flows or lines differs from the stage's periods,
THEN it SHALL return `FM-E025` naming the stage, the expected and the actual count." The #110
plan assigns this to #38 ("a `given` stage accepts build-up lines through a new optional field
`lines` next to `flows` (exactly one of the two), resolved by `check_stages` into a new
`StageKind` variant that becomes amounts before projection"). Implemented as follows:
- `Stage.lines: Option<Vec<FreeCashFlowLines>>`, exactly one of `flows` and `lines` on a
  `given` stage (`FM-E001`), refused on a generated stage (`FM-E001`); a count different from
  the periods is `FM-E025` naming `/stages/i/lines`.
- `dcf.fcff` gains the optional `notional_tax_rate` and `negative_ebit`; they are refused when
  no stage has `lines`. `interest_tax_rate` is not a field of `dcf.fcff`: interest expense and
  net borrowing are refused in the lines of the enterprise basis, because FCFF is before
  financing. `dcf.fcfe` (#165) adds `interest_tax_rate` and takes the FCFE of the lines; the
  engine already selects the FCFE on the equity basis.
- `check_stages` resolves the stage to `StageKind::Lines`; `build_up_stages` calls `build_up`
  with `Location { "/stages/i/lines", first period of the stage }` and replaces it with
  `StageKind::Given` of the built-up flows before projection. The steps of the build-up are in
  the explain with the prefix "Build-up:"; its conventions are reported once, prefixed
  "free cash flow build-up (dcf.free_cash_flows v1):"; the period table says the flows were
  built up from line items.
- Schema: additive to `dcf.fcff` version 3 (`scripts/schema_snapshots.py --diff-against` the
  schemas of main: "Schema changes are additive, new versions, or none"); version 3 is not
  released, no version bump.
- Golden cases `tests/golden/dcf.fcff/stage-with-build-up-lines*.yaml` from the two 50-digit
  references composed (`reference_free_cash_flows.stages_with_flows`, then
  `reference_dcf.dcf_fcff`), and error cases; a unit test checks that lines value exactly
  like the built-up flows.

### Explain
Per period: taxes on EBIT (formula with the rate substituted, or "given"), EBIT after taxes,
each labelled adjustment and their sum, FCFF with every component substituted, and for FCFE
the after-tax interest and the FCFE. Conventions (R28): tax form and rate, negative EBIT
treatment and the periods where it applied, no tax loss carry-forward, the rate of the
interest tax shield, the sign conventions of the tax amounts (amounts form), ΔNWC, net
borrowing and adjustments, amounts per period in currency units and not discounted. Negative
values appear in parentheses in every formula.

### Tests
- Independent reference `scripts/reference_free_cash_flows.py` (Python `decimal`, local
  50-digit context, written from R2 to R4, no shared code).
- Golden cases `tests/golden/dcf.free_cash_flows/`: Damodaran, *Investment Valuation*, 2nd
  edition, Illustration 15.7 (Boeing) in the amounts form from the published EBIT (1 − tax
  rate) of 1,138, with the 1,139.45 discrepancy recorded in the file (AC1, Decision 12);
  multi-period FCFF and FCFE against the reference, both tax forms, negative EBIT under
  `credit` and `zero` (AC3), adjustments; error cases for each negative D&A, capex and interest
  and for every tax-form conflict (AC3).
- Contract cases `tests/contracts/dcf.free_cash_flows.yaml`.
- Rust unit tests: FCFE equals FCFF with zero interest and zero net borrowing (AC2), every
  error message contains a JSON pointer and the period (AC14), every convention is reported.
- Python: seeded random comparison with the reference (to the 28th significant digit), and the
  AC2 property on random inputs.

### Schema changes
Additive: new method `dcf.free_cash_flows` version 1, snapshot
`schemas/dcf.free_cash_flows/v1.json`, entry in `schemas/index.json` with
`introduced_in: unreleased`. `dcf.fcff` version 3 gains the optional `stages[].lines`,
`notional_tax_rate` and `negative_ebit` (additive, see above).

### Documentation
`docs/en/methods/dcf-free-cash-flows.md` and the Italian edition
`docs/it/methods/dcf-free-cash-flows.md` (terms from `i18n/glossary.yaml`; English practice
terms kept where the spec marks the Italian term "to verify": EBIT, net borrowing, operating
taxes), nav entry, methods index row (status preview), Python helper
`fairmark.dcf.free_cash_flows`, release note `changes/38.feature.md`.
