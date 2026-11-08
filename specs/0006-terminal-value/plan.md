# SPEC-0006 — Plan

Status: active (SPEC-0006 approved on 2026-11-07); changes to the spec are reflected here first.
Issue: #115 (terminal value building blocks `tv.*`).

## Design

### Module layout in `crates/fairmark-core`
A new theme module `tv` (ADR 0009: the type of `tv.x` lives in `fairmark_core::tv`).

| File | Content |
| --- | --- |
| `src/tv/mod.rs` | Shared pieces: growth cap check (R8), domain checks, `TerminalBasis`, `Line` (labelled amount) |
| `src/tv/gordon.rs` | `Gordon` (R1) |
| `src/tv/value_driver.rs` | `ValueDriver` (R2) |
| `src/tv/h_model.rs` | `HModel` (R3) |
| `src/tv/exit_multiple.rs` | `ExitMultiple` (R4) |
| `src/tv/limited_horizon.rs` | `LimitedHorizon` (R5), reusing `tvm::annuity` |
| `src/tv/normalised_income.rs` | `NormalisedIncome` (R6) |
| `src/tv/liquidation.rs` | `Liquidation` (R7) |
| `src/tv/compare.rs` | `Compare` (R9), typed composition of the seven functions above |
| `src/error.rs` | Two variants: `FM-E018` terminal value input outside its domain, `FM-E019` enterprise and equity measures mixed (provisional numbers, see Risks) |
| `src/registry.rs` | Eight lines, in a `tv` group between `coc` and `dcf` |

Every method returns the terminal value at the horizon N, not discounted (Scope), in a field
named `terminal_value`, plus the assumptions it implies. Each method has a public function
`tv::<name>(&Input) -> Result<(Output, Explain)>` used by `tv.compare` and, later, by
SPEC-0001 (typed composition, ADR 0009; not the registry path of ADR 0010).

### Inputs (all rates are decimal fractions per period of the terminal phase)
| Method | Fields |
| --- | --- |
| `tv.gordon` | `rate`, `growth`, exactly one of `next_flow` (CF_{N+1}) and `last_flow` (CF_N), optional `growth_cap` |
| `tv.value_driver` | `nopat_next`, `growth`, `ronic`, `rate`, optional `growth_cap` |
| `tv.h_model` | `last_flow`, `short_term_growth`, `long_term_growth`, `half_life`, `rate`, optional `growth_cap` |
| `tv.exit_multiple` | `multiple`, `metric` (`ebitda`, `ebit`, `revenue`, `earnings`), `metric_value`; optional `rate` with `next_flow` (both or neither); optional `growth_cap` (needs `rate` and `next_flow`) |
| `tv.limited_horizon` | `flow` (first flow, at N + 1), `rate`, `periods` (n), optional `growth` (default 0), optional `residual_value` at N + n (default 0), optional `growth_cap` |
| `tv.normalised_income` | `income`, `capitalisation_rate`, optional `growth` (default: none, R / i), optional `growth_cap` |
| `tv.liquidation` | `assets`, `liabilities`, `liquidation_costs` (lists of `{label, value}`), `tax_on_gains` (amount) |
| `tv.compare` | `rate`, `basis` (`enterprise` or `equity`), optional `growth_cap`, `techniques`: a list of `{"tv.<name>": {terms}}` where the terms are the method's own fields without `rate` and `growth_cap` |

Choices that follow from the spec text and the maintainer decisions:
- **Flow at N + 1 (R1, conventions table: "required choice").** Two optional fields, exactly
  one required; giving both or neither is `FM-E001`. The choice is reported in
  `explain.conventions` and in the output field `flow_basis`.
- **Growth cap (R8).** Respected when g ≤ cap, exceeded when g > cap; reported in the output
  field `growth_cap` (`{growth, cap, status}` with status `respected` or `exceeded`, `null`
  when no cap is given) and in `explain.conventions` ("none given" when absent). Never an
  error. The growth checked is the perpetual one: g (Gordon, value driver), g_l (H-model,
  where g_s is transient by construction), the implied growth (exit multiple), the effective
  growth (limited horizon, normalised income: 0 when not given).
- **RONIC = r (R2, AC2).** Computed directly as NOPAT_{N+1} / r, so the result is independent
  of g in decimal arithmetic too; the output field `value_of_growth` reports `none`, and
  `creates` (RONIC > r) or `destroys` (RONIC < r) otherwise.
- **Exit multiples (R4, decision 2).** `ebitda`, `ebit`, `revenue` are enterprise measures,
  `earnings` (P/E) an equity measure; the output reports `basis`. `book_value` (in the R4
  list and the conventions table) is not implemented: its basis is not settled by the
  decision (open question 1).
- **Limited horizon (R5, decision 1).** Constant (or optionally growing) flow for n explicit
  periods at the end of each period N + 1 ... N + n, residual value at N + n; the value at N is
  `tvm::annuity` (ordinary) plus the residual value × (1 + r)^−n. The declining flow and the
  goodwill decay are later options (decision 1), not implemented.
- **Tax on gains (R7).** Read literally as a given amount, like the liquidation costs.
  Computing it from a tax rate and tax bases is open question 2.
- **Compare (R9).** Consistency is by construction: the terminal-phase rate and the growth
  cap are given once and passed to every technique that uses them; `basis` is declared once
  and an exit multiple of the other basis is `FM-E019` (decision 2: never mixed). The
  capitalisation rate of `tv.normalised_income` stays a technique term (open question 3).
  The first technique that fails returns its error; there are no partial results.

### Errors (R10)
| Condition | Code |
| --- | --- |
| g ≥ r (Gordon, value driver, H-model on g_l), g ≥ i (normalised income) | `FM-E004` (existing) |
| RONIC ≤ 0; multiple ≤ 0; metric ≤ 0; n < 1; H < 0; a negative asset value, liability, cost or tax on gains | `FM-E018` (new) |
| Exit multiple of one basis inside a comparison declared on the other | `FM-E019` (new) |
| Missing or conflicting flow fields, a growth of −100% or lower (as in `tvm.perpetuity`), empty label, empty technique list | `FM-E001` (existing) |
| The rate of the limited horizon at or below −100% | `FM-E003` (existing, from `tvm.annuity`) |

A multiple or metric of zero is refused with the negative ones: it gives a terminal value of
zero, which is not a valuation, and makes the implied growth undefined.

## Schema changes
Additive only: eight new methods at version 1, `schemas/tv.<name>/v1.json` and eight entries
in `schemas/index.json`. No existing method version changes. Optional fields and defaults are
documented in the schema (`growth`, `residual_value`, `growth_cap`, `rate`, `next_flow`).

## Numerical approach
- Only rational operations through `crate::num` (checked), plus `rates::pow` for (1 + r)^−n
  in the limited horizon (integer exponent) and the annuity factor of `tvm::annuity`.
- Gordon: CF_{N+1} = CF_N × (1 + g) when the last flow is given, then one division.
- Value driver: rr = g / RONIC, FCF_{N+1} = NOPAT × (1 − rr), TV = FCF_{N+1} / (r − g);
  RONIC = r: TV = NOPAT / r.
- H-model: stable part CF_N (1 + g_l) / (r − g_l), extra-growth part CF_N H (g_s − g_l) /
  (r − g_l), summed.
- Exit multiple: TV = multiple × metric; implied g = r − CF_{N+1} / TV.
- Normalised income: R / (i − g), with g = 0 when not given (written R / i in explain).
- Liquidation: sums of labelled lines, then one subtraction per total.

## Test strategy
| Test | Source of expected values | Proves |
| --- | --- | --- |
| Golden `tv.value_driver/damodaran-*.yaml` | Damodaran, "The Big Assumption" table (NYU Stern valuation lectures, spring 2020, session 11), at its published precision (nearest unit) | AC1, AC2 |
| Golden `tv.gordon/damodaran-fcff-from-roc-table.yaml` | Same table, the g = 3%, ROC = 12% cell restated as a Gordon flow FCFF = 100 × (1 − 0.03 / 0.12) = 75 | AC1 |
| Golden cases of every method, exact | `scripts/reference_tv.py` (Python `decimal`, 50 digits, local context), written from the spec formulas | R1 to R7, R9 |
| Golden error cases | spec R10 | AC6 |
| Rust unit tests in each module | constructed grids | AC2 (RONIC = r, several g), AC3 (g_s = g_l equals Gordon), AC4 (implied growth fed to Gordon), AC5 (the longest horizon within the decimal range is within 1e-20 of the perpetuity; see open question 6) |
| Contract cases `tests/contracts/tv.*.yaml` | constructed | extreme inputs never panic; conventions reported |
| `python/tests/test_reference_tv.py` | reference script against the engine on small grids | independence of the golden values |

## Documentation
- Methodology pages `docs/en/methods/tv-*.md` (eight), nav entries, index rows; Italian edition
  in `docs/it/methods/` with the glossary; `docs/en/reference/errors.md` and its translation.
- Python helpers in `python/fairmark/tv.py` (translate only), exported in `__init__.py`.
- Release note fragment `changes/115.feature.md`.

## Risks and alternatives
- **Error code collision.** Another branch adds codes in parallel; `FM-E018` and `FM-E019` are
  provisional and may be renumbered at merge, before release.
- **Compare with full inputs per technique** (each technique repeating `rate`, checked for
  equality) was rejected: a shared field cannot disagree, an equality check can be forgotten.
- **Compare through the registry** (ADR 0010) was rejected: the techniques are known at compile
  time, so typed composition (ADR 0009) applies.
- **Internally tagged technique list** (`{"method": "tv.gordon", ...}`) was rejected: serde
  buffers internally tagged content, which loses exact JSON numbers; the externally tagged
  form keeps them.

## Open questions for the maintainer
1. `book_value` as an exit-multiple metric: equity basis (P/BV) or enterprise basis (EV over
   book value of invested capital)? Not implemented until decided.
2. Tax on gains in `tv.liquidation`: keep it as a given amount, or compute it from a tax rate
   and the tax basis of each asset (and then: are losses netted against gains)?
3. In `tv.compare`, should `tv.normalised_income` use the shared terminal-phase rate as its
   capitalisation rate, or keep its own (current choice)?
4. In `tv.compare`, which basis does `tv.liquidation` carry? Currently the analyst declares the
   basis and chooses the liabilities accordingly (financial debt included for equity).
5. AC1 for `tv.gordon` and `tv.value_driver` uses one published table; a second published
   stable-growth example with more digits would strengthen it.
6. `tv.limited_horizon` reuses `tvm.annuity` (R5), which also computes the future value at
   N + n; for long horizons that value leaves the decimal range and the method returns
   `FM-E008` (with a flow of 100 at 10%, beyond about 620 periods). AC5 is therefore shown at
   600 periods, within 2e-22 of the perpetuity. Should `tvm` expose the annuity factor without
   the future value, so that the finite horizon accepts any n up to 10,000?
