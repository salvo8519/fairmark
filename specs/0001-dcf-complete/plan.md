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
6. Terminal discounting: `horizon` uses DF_N; `mid_period` uses DF_N × (1 + r_T)^0.5 with
   the terminal rate r_T (R21, Decision 16; with one rate r_T = r_N); the present value is
   TV_N / G_N, or TV_N × (1 + r_T)^0.5 / G_N.
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
  axis (today t = k or k − 0.5) becomes the dated axis of `tvm::time_axis`. The next version of
  `dcf.fcff`, planned with this issue, also adds the required measurement basis of the
  non-operating assets (spec, Decision 21).

## Issue #43 — Terminal value technique and discounting time
Extends version 3 additively through the extension point above.

| Item | Design |
| --- | --- |
| Module | `src/dcf/terminal.rs` holds the terminal input types (moved from `engine.rs`, same schema), the checks and the computation; `engine.rs` calls it at the two places it called the Gordon code, so that the parallel issue #38 (stages) and this one touch disjoint parts of the engine |
| Techniques | Optional properties of `terminal`: `tv.gordon` (#110), `tv.h_model`, `tv.value_driver`, `tv.exit_multiple`, `tv.limited_horizon`, `tv.normalised_income`, `tv.liquidation`; exactly one present, checked at run time (`FM-E001` naming the keys) |
| Rate | The terminal rate (the single rate, or `terminal_wacc`) is passed to every technique that takes a rate (R19, Decision 13), including the capitalisation rate of `tv.normalised_income`, which accepts no separate rate inside the DCF (the SPEC-0006 decision on its own capitalisation rate concerns `tv.compare` only); `tv.exit_multiple` uses it only for the implied growth when `next_flow` is given; `tv.liquidation` takes none |
| Terms | The technique's own terms (SPEC-0006), the rate and the last explicit flow excluded; a growth cap where the technique accepts one (R22). Terms that the standalone method defaults (`growth` and `residual_value` of `tv.limited_horizon`, `growth` of `tv.normalised_income`) are required inside the DCF (R29) |
| First terminal flow | `tv.gordon`: `last_flow_grown` or `normalised_next_flow` (#110). `tv.h_model` (Decision 15): `first_flow` required, its own enumeration `HModelFirstFlow`: `last_flow_grown` passes the last explicit flow as CF_N; `normalised_base_flow` passes the given `normalised_base_flow` as CF_N, standard formula, no conversion from N + 1; the amount is required with that choice and refused otherwise (`FM-E001`) |
| Basis (R19) | `tv.exit_multiple`: the basis of the metric; `tv.normalised_income`, `tv.liquidation`: the declared basis; each must equal the basis of the DCF, otherwise `FM-E019` naming the field and the fix. The flow-based techniques take the basis of the flows they are given |
| Discounting (R21, Decisions 6, 16, 17) | DF_N, or DF_N × (1 + r_T)^0.5 at N − 0.5 with the terminal rate r_T (Decision 16; no change with one rate). Warnings: `tv.exit_multiple`, `tv.liquidation` or `tv.limited_horizon` (its residual value, Decision 17) discounted at N − 0.5 (a price at a date); a perpetual-growth technique (`tv.gordon`, `tv.h_model`, `tv.value_driver`, `tv.normalised_income`) discounted at N under mid-period timing |
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
- **R11 with `tv.h_model` (#43).** The literal comparison is with the perpetual growth g_l; the
  growth right after N in the H-model is g_s. Implemented literally (g_l); g_s may be the
  comparison the maintainer intends.

Resolved on 2026-11-08 by the maintainer (spec, Decisions 15 to 17): the first terminal flow of
`tv.h_model` is the last explicit flow or a normalised base flow at N; the half-period shift at
N − 0.5 uses the terminal rate; `tv.limited_horizon` discounted at N − 0.5 carries the warning
of a price at a date.

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
- FCFE_t = FCFF_t − (interest_t − shield_t) + net borrowing_t, with shield_t = t ×
  interest_t, t = `notional_tax_rate` in the rate form and `interest_tax_rate` in the amounts
  form (R3). In a period where the `zero` treatment applies, t = 0: no interest tax shield, the
  interest is subtracted in full and the FCFE equals the net-income route (amendment 14 of the
  spec, approved by the maintainer on 2026-11-08). Since Decision 22 (version 2), under `zero`
  shield_t = t × min(interest_t, max(EBIT_t, 0)) in every period, so the total taxes never
  fall below zero. The rate applied in each period (`interest_tax_rate`) and the shield
  (`interest_tax_shield`) are in the output; the shield is an explain step and a binding limit
  is reported in `explain.conventions`.
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
  "free cash flow build-up (dcf.free_cash_flows v<N>):", N the version of the build-up; the
  period table says the flows were built up from line items.
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

## Issue #166 — Nominal and real terms: basis check and rate conversion

Requirements: R14 to R18 (Decision 5), with R13 for converted rates, R27 and R28 for the new
fields, AC6 (inflation list count), AC8, AC9 and the warning of AC10 (growth below inflation).
Extends version 3 additively through the extension point above: only optional input fields are
added, and output fields are added next to the existing ones.

### Input (`dcf.fcff` version 3, additive)
| Field | Type | Rule |
| --- | --- | --- |
| `inflation` | decimal, optional | expected inflation per period, decimal fraction, greater than -1; applies to every explicit period and to the terminal phase |
| `inflation_per_period` | list of decimals, optional | one per period of the explicit horizon, exactly N (`FM-E025` otherwise); requires `terminal_inflation` |
| `terminal_inflation` | decimal, optional | expected inflation of the terminal phase; required with `inflation_per_period`, refused with `inflation` and alone |

Exactly one form or none; any other combination is `FM-E001` naming the fields by their JSON
pointers and stating the fix (Decision 18); an inflation of -1 or lower is `FM-E003` naming its
pointer, the code of `rates.real_nominal` (Decision 19). The form mirrors the rate forms of
R12 and Decision 13 (one value for every period and the terminal phase, or one per period plus
a terminal value), so that the two lists read the same way. The forms are checked whenever
they are given, also when the terms are equal (R18).

### Computation (engine, `check_terms` replaced by `resolve_terms`)
After the rates are resolved (R12, R13) and before the terminal technique is checked:
1. Terms equal: no conversion (`rate_used` = `discount_rate`); with inflation, R18: it is used
   only for R17 and the conventions say that no conversion was made.
2. Terms differ, no inflation: `FM-E024` as before (R15), the message naming both terms fields,
   the inflation fields and the two fixes.
3. Terms differ, inflation given (R16, Decision 5): every rate of the explicit periods with the
   inflation of its period, and the terminal rate with the inflation of the terminal phase
   (`inflation`, or `terminal_inflation`), is converted to the terms of the cash flows through a
   typed call to `rates::real_nominal` (`to_real` for real flows, `to_nominal` for nominal
   flows); its step formula, with the values substituted, becomes the explain step "Period k:
   discount rate used" and "Terminal rate used". Flows and growth rates are never converted.
   A converted rate of -1 or lower would be `FM-E003` naming the period (R13); with both inputs
   above -1 the exact relation cannot produce one, the check stays as a guard.
4. The converted rates feed `tvm::period_discount_factors`, the compounding factors, the
   terminal technique (its own check, so that a perpetual growth not below the converted
   terminal rate is `FM-E004` naming the rate field "converted with the inflation field") and
   the half-period shift of Decision 16.
5. R17, whenever inflation is given: the perpetual growth stated by the terminal technique (the
   same growth as the continuity check of R11: `growth` of `tv.gordon`, `tv.value_driver`,
   `tv.normalised_income`; `long_term_growth` of `tv.h_model`) is restated in the other terms
   with the inflation of the terminal phase, again through `rates::real_nominal`; for nominal
   flows a growth below that inflation is reported as a warning (a real decline). For the
   techniques that state no perpetual growth (`tv.exit_multiple`, `tv.limited_horizon`,
   `tv.liquidation`) a convention says that none is restated (open question below).

### Output (additive)
- `periods[].rate_used`: the converted rate (already in the table since #110);
  `periods[].inflation`: the inflation used to convert the rate of the period, `null` when no
  conversion is made.
- `terms`: `cash_flow_terms`, `discount_rate_terms`, `conversion` (`none`, `to_real`,
  `to_nominal`), `terminal_rate_given`, `terminal_rate_used` (equal to `terminal.rate`),
  `terminal_inflation` (the inflation of the terminal phase, `null` without inflation),
  `perpetual_growth` (in the terms of the cash flows) and `perpetual_growth_other_terms` (R17;
  `null` without inflation or without a perpetual growth).

### Explain (R28)
Conventions: the terms on both sides; the inflation form and values; the conversion with its
direction, the exact Fisher relation and the building block (`rates.real_nominal v1`), the
statement that no cash flow or growth rate is converted; R18 "no conversion"; R17 growth in the
other terms and the warning. Steps: "Period k: discount rate" (as given), "Period k: expected
inflation", "Period k: discount rate used" with the substituted Fisher formula, "Terminal
rate: expected inflation", "Terminal rate used", "Perpetual growth in real (nominal) terms".

### Tests
- Reference `scripts/reference_dcf.py` extended: the rate conversion restated from R16
  ((1 + r) / (1 + i) − 1, (1 + r)(1 + i) − 1), R17 growth, `rate_used` and `terms` outputs; a
  separate function inflates real flows to nominal flows for AC9 (end of period, 1e-20
  relative).
- Golden cases (new files only) `tests/golden/dcf.fcff/terms-*.yaml`: real flows with a
  nominal rate and inflation; nominal flows with a real rate and inflation; inflation per period
  with rates per period and a terminal rate under mid-period timing at N − 0.5; terminal growth
  in the other terms with equal terms (R18); a converted rate equal to a real rate given
  directly (identical expected values, the inflation chosen so that the conversion is exact);
  errors `error-terms-*.yaml` (count, forms, inflation of -1, growth not below the converted
  terminal rate).
- Python `python/tests/test_reference_dcf.py`: grid against the reference with conversion;
  AC9 exact equality with the rates converted by `rates.real_nominal` given directly, and the
  inflated-flows equivalence within 1e-20 relative.
- Rust unit tests: every new error with its JSON pointer, every convention, the R17 warning,
  no conversion with equal terms.

### Resolved by the maintainer (issue #166)
Approved on 2026-11-08 (spec, "Decisions"):
- **R17 for techniques without a stated perpetual growth**: nothing is restated for
  `tv.exit_multiple`, `tv.limited_horizon` and `tv.liquidation`, and a convention says so; the
  implied growth of an exit multiple is an output, not restated (Decision 20 a).
- **Inflation of the R17 warning with inflation per period**: the inflation of the terminal
  phase (Decision 20 b).
- **R17 for real flows**: no warning; the restated nominal growth is reported (Decision 20 c).
- **Terminal inflation next to one inflation**: refused with `FM-E001` naming both fields
  (Decision 18).
- **Inflation at or below -1**: `FM-E003`, the code of `rates.real_nominal` (Decision 19).

## Issue #44 — From value of operations to equity value

Requirements: R23 and R25 for the enterprise basis (`dcf.fcff`), R26 (already on main), R27 and
R28 for the fields of the bridge; acceptance criterion AC13 for `dcf.fcff` (the FCFE part of
AC13 and R24 belong to `dcf.fcfe`, #165). Extends version 3 additively through the extension
point above; edits stay in `src/dcf/fcff.rs` (the engine is not touched).

### Input (additive to version 3)
| Field | Type | Rule |
| --- | --- | --- |
| `debt_like_items` | list of `{label, value, measurement_basis, basis_note?}`, optional | each value zero or more, subtracted; labels not empty; basis required; note required with `other`, never empty |
| `non_controlling_interests` | list of `{label, value, measurement_basis, basis_note?}`, optional | each value zero or more, subtracted; labels not empty; basis required; note required with `other`, never empty |

One new type, `BridgeClaim`, for both lists: a claim on the value of the company other than the
net financial position. `measurement_basis` is the required enumeration `MeasurementBasis`
(`book_value`, `market_value`, `fair_value`, `present_value`, `other`), with an optional
`basis_note` that is required with `other` (Decision 21); both are echoed in `explain` and never
used in the calculation. The engine does not choose the basis: the analyst states it and the
result shows it next to the amount. The first draft of this plan used a free text; the
maintainer replaced it with the enumeration (Decision 21). The net financial position keeps the
SPEC-0007 structure unchanged (R23, SPEC-0007 R6); `non_operating_assets` keeps the shape of
main (`tv::Line`) in version 3 (see "Resolved" below).

### Computation
equity value = enterprise value − net financial position − Σ debt-like items − Σ
non-controlling interests, in `Decimal` through `crate::num` (checked). Every line is an
`explain` step with its amount ("as given", with the label and the basis in the step label),
each list has a total step with the values substituted, and the equity value step substitutes
every subtracted total. Empty lists are reported as "none given" (R25).

### Errors (R27)
`FM-E001`, before any calculation, with the JSON pointer and the fix:
`/debt_like_items/i/value` or `/non_controlling_interests/i/value` negative ("zero or more;
an asset belongs in `non_operating_assets`"), `/…/i/label` or `/…/i/measurement_basis` empty.
No new error code.

### Conventions (R28)
Bridge items measured at the valuation date, as given (existing); signs: non-operating assets
added, net financial position (positive for net debt), debt-like items and non-controlling
interests subtracted, each item zero or more; the stated measurement basis of each debt-like
item and each non-controlling interest; "debt-like items: none given", "non-controlling
interests: none given"; the R26 warning unchanged.

### Output
`bridge` gains `debt_like_items`, `debt_like_items_total`, `non_controlling_interests`,
`non_controlling_interests_total`; the description of `equity_value` names the four terms.
Output fields are not part of the schema snapshots.

### Tests
- `scripts/reference_dcf.py`: the equity value restated with both lists (50 digits).
- Golden cases (new files only): `bridge-every-line.yaml` (net financial position with other
  items, two non-operating assets, two debt-like items, one non-controlling interest; reference);
  `bridge-negative-equity-value.yaml` (claims above the enterprise value, equity value negative
  and not floored; the warning is checked by a unit test and a contract case, because golden
  cases check values only); `damodaran-segovia-consolidated.yaml` (Damodaran, *Investment
  Valuation*, 2nd edition, chapter 16, Illustration 16.6, consolidated approach: 2,205 − 500 −
  122.5 + 22.5 = 1,605); error cases `error-bridge-*.yaml`.
- Rust unit tests: every error with its pointer (AC14), conventions reported, the bridge
  property EV − NFP − Σ DLI − Σ NCI = equity value.
- Python: reference grid with random bridges against the engine; the property above.

### Resolved by the maintainer on 2026-11-08 (spec, Decision 21)
- **Measurement basis.** The free text is replaced by the required enumeration
  `MeasurementBasis` (`book_value`, `market_value`, `fair_value`, `present_value`, `other`) with
  an optional `basis_note`, required and not empty with `other` (`FM-E001` naming
  `/…/i/basis_note`).
- **Negative non-controlling interests.** Stay refused (R25, "zero or more").
- **Double counting with the net financial position.** No duplicate-label warning: covered in
  the documentation only (R25, SPEC-0007 R6: the engine does not choose the items).
- **Measurement basis of non-operating assets.** Non-operating assets keep the shape of tv::Line
  in dcf.fcff version 3; a required measurement basis for them is added at the next version of
  dcf.fcff (planned with dated periods, issue #167), because adding it now would break the v3
  schema or change the shared tv::Line type. Approved by the maintainer on 2026-11-08.
  (`scripts/schema_snapshots.py --diff-against` main reported
  `v3 dcf.fcff.non_operating_assets[]: '$ref' changed`.)

## Issue #165 — Equity DCF on the shared engine (`dcf.fcfe`)

Requirements: R1 (the equity method id and version), R3 and R7 for `given` stages with
build-up lines on the equity basis, R19 (basis check on the equity basis), R22 (terminal value
weight), R24, R25, R26, R27, R28 and R29 for the fields of the method; acceptance criteria AC11
(EV/EBITDA with FCFE returns `FM-E019`) and the FCFE part of AC13. Everything the engine does
for `dcf.fcff` (stages, rates, terms and inflation, timing, terminal techniques) applies
unchanged: the method is a second caller of the engine, as planned in "Extension points".

### What the spec decides
- Basis and rate: "an equity DCF (`dcf.fcfe` version 1) that discounts FCFE at the cost of
  equity and returns equity value" (R1); the basis is reported (R1, R28).
- Equity value: "`equity_value` = present value of the explicit FCFE + present value of the
  terminal value + Σ non-operating assets − Σ debt-like items − non-controlling interests, and
  SHALL NOT return `value_of_operations` or `enterprise_value`; IF a net financial position is
  given, THEN it SHALL return `FM-E019` (debt is already in the flows)" (R24, Decision 9).
- Cash: the net financial position, which nets cash against debt, is refused (R24). Cash that
  the FCFE does not already reflect (cash not needed for operations, whose interest income is
  not in the flows) is a non-operating asset, as in [S1] ("Non-operating assets, such as
  excess cash and marketable securities"); the engine does not choose the items (R25). The
  methodology page says so; the engine reports "non-operating assets: none given" when none is.
- Terminal value weight: "the present value of the terminal value divided by the present value
  of the explicit flows plus the present value of the terminal value" (R22).
- Terminal basis: "IF the technique's basis differs from the basis of the DCF (enterprise for
  FCFF, equity for FCFE), THEN it SHALL return `FM-E019`" (R19); AC11: "EV/EBITDA with FCFE"
  returns `FM-E019`. Already implemented by `terminal::check` for the exit multiple, the
  normalised income and the liquidation; "the flow-based techniques take the basis of the flows
  they are given" (this plan, issue #43): `tv.gordon`, `tv.h_model` and `tv.limited_horizon`
  on the FCFE.
- Lines: "`dcf.fcfe` (#165) adds `interest_tax_rate` and takes the FCFE of the lines; the
  engine already selects the FCFE on the equity basis" (this plan, issue #38): FCFE_t = FCFF_t −
  interest_t × (1 − t) + net borrowing_t (R3, amendment 14).
- Rate fields: `cost_of_equity`, `cost_of_equity_per_period`, `terminal_cost_of_equity`
  mapped to the engine's `RateInput` with their own `RateFields` (this plan, "Extension
  points").

### Module layout in `crates/fairmark-core`
| File | Content |
| --- | --- |
| `src/dcf/bridge.rs` (new) | The bridge types and checks shared by both bases, moved from `fcff.rs` without change: `BridgeClaim`, `MeasurementBasis`, the checks of labelled amounts and claims, the claim totals, the non-operating assets total and the negative equity warning (R25, R26). Same type names, so the JSON Schema of `dcf.fcff` version 3 is unchanged |
| `src/dcf/fcff.rs` | Uses `bridge.rs`; the strings, the order of the steps and conventions and every number stay identical (all `dcf.fcff` golden and contract cases unchanged) |
| `src/dcf/fcfe.rs` (new) | `DcfFcfeInput`, `DcfFcfeOutput`, `FcfeBridge`, `pub fn fcfe`, unit type `Fcfe` (`dcf.fcfe`, version 1, preview); unit tests |
| `src/dcf/engine.rs` | On the equity basis only, the build-up convention states the FCFE formula as well and the period step points to the FCFE step of the build-up; the enterprise path is unchanged |
| `src/dcf.rs`, `src/registry.rs` | One module line and re-exports; one registry line `dcf::Fcfe` |
| `src/error.rs` | Unchanged: `FM-E019` already covers the bridge on the equity basis (Decision 10) |

### Input (`dcf.fcfe` version 1)
The fields of `dcf.fcff` version 3 with the same meaning, except:

| Field | Rule |
| --- | --- |
| `cost_of_equity` or `cost_of_equity_per_period` + `terminal_cost_of_equity` | Exactly one form, as the WACC fields of `dcf.fcff` (R12, Decision 13) |
| `base_flow` | FCFE of the base period CF_0 |
| `interest_tax_rate` | For stages with `lines` and `operating_taxes`: the rate t of the interest tax shield (R3); refused when no stage has lines |
| `net_financial_position` | Accepted by the schema only to be refused with `FM-E019` naming the field and the fix (R24, AC13) |
| `non_operating_assets`, `debt_like_items`, `non_controlling_interests` | As in `dcf.fcff` version 3, same types (`tv::Line`, `BridgeClaim`) |

### Output
`basis` (`equity`), `equity_value`, `pv_explicit_period`, `terminal_value_weight` (`null` when
the denominator is zero), `periods`, `terms`, `terminal`, and `bridge`:
`pv_explicit_and_terminal` (present value of the explicit FCFE plus present value of the
terminal value, the starting line of R24), the non-operating assets with their total, the
debt-like items and the non-controlling interests with their totals, `equity_value`. No
`value_of_operations` and no `enterprise_value` (R24).

### Explain (R28)
Conventions: "basis: equity, free cash flows to equity (FCFE) discounted at the cost of
equity"; that no net financial position is subtracted because debt is in the flows (R24); the
bridge signs of the equity basis; "none given" for each empty list; the measurement basis of
each claim; the R26 warning. Steps: every line of the bridge, the starting line, the equity
value with every term substituted, the terminal value weight. Every convention of the engine
(terms, rates, timing, terminal technique, build-up) is reported as for `dcf.fcff`.

### Tests
- `scripts/reference_dcf.py`: `dcf_fcfe`, written in its own function from R22 and R24 (rates
  from the cost of equity fields, the explicit and terminal present values by the reference's
  own discounting, no net financial position).
- Golden cases `tests/golden/dcf.fcfe/` (new folder): reference cases (three stages with rates
  per period and an equity bridge; a stage of build-up lines with interest and net borrowing;
  exit multiple on earnings; terms with inflation under mid-period timing at N − 0.5; negative
  equity value); error cases (`FM-E019` for a net financial position, EV/EBITDA, a normalised
  income and a liquidation declared on the enterprise basis; lines without financing lines;
  `interest_tax_rate` without lines; the WACC field; cost of equity forms and counts).
  A published Damodaran FCFE illustration re-derived (Investment Valuation, 2nd edition,
  chapter 14) when its text can be read (see the open questions).
- Python `python/tests/test_reference_dcf_fcfe.py`: golden values equal the reference;
  engine against the reference on a grid; the consistency property: with no debt and no net
  borrowing, `dcf.fcfe` on the FCFE equals the `dcf.fcff` equity value on the same flows at
  the same rate, bit for bit, with the same bridge items.
- Rust unit tests: every error with its JSON pointer (AC14), every convention, no boolean in
  the schema, the R24 identity, the R26 warning.
- Contract cases `tests/contracts/dcf.fcfe.yaml`.

### Schema changes
Additive: new method `dcf.fcfe` version 1, snapshot `schemas/dcf.fcfe/v1.json`, entry in
`schemas/index.json`. `dcf.fcff` version 3 unchanged (`scripts/schema_snapshots.py
--diff-against` the schemas of main).

### Documentation
`docs/en/methods/dcf-fcfe.md` and the Italian edition, nav and methods index (status preview),
`docs/en/reference/errors.md` (`FM-E019` on the equity basis), Python helper `fm.dcf.fcfe`,
release note `changes/165.feature.md`, `specs/method-evidence.yaml`, Italian method title.

### Open questions for the maintainer (issue #165)
- **`tv.value_driver` on the equity basis.** The spec does not decide it. Its terms are
  NOPAT_{N+1} and RONIC, enterprise measures; the equity counterpart is net income and the
  return on equity, with the equity reinvestment rate g / ROE. Options: (a) refuse it on the
  equity basis with `FM-E019` and point to `tv.gordon` with a normalised next flow; (b) accept
  it with the same fields read as net income and return on equity; (c) add equity terms
  (`net_income_next`, `return_on_equity`) as optional fields, required on the equity basis.
  Recommendation: (a) in version 1, (c) later as an additive option. Until decided, the engine
  refuses it on the equity basis (no number is produced, so any option remains open).
- **Damodaran FCFE illustration.** The source named by the spec (Illustration 14.5) could not
  be read in the implementation session; the golden case is added when its inputs and
  published values are checked against the text.
