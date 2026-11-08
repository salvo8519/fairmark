# SPEC-0007 — Plan

Status: active (SPEC-0007 approved on 2026-11-07); changes to the spec are reflected here first.
Issue: #114. This plan covers the parts of the spec whose definitions are unambiguous; the parts
that need a maintainer decision are listed under "Open questions" and are not implemented.

## Design

### Module layout in `crates/fairmark-core`
| File | Content |
| --- | --- |
| `src/ratios/mod.rs` | Shared pieces: `Balances` (closing, average), `DayBasis` (360, 365), `RatioValue` (value or reason), the `Ratio` trait (ADR 0009, point 5) with its two implementors `Quotient` and `DayRatio`, the balance resolver, `NetFinancialPosition` with its composition |
| `src/ratios/profitability.rs` | `Profitability` method, `ratios.profitability` (R1, R2, R3) |
| `src/ratios/capital_structure.rs` | `CapitalStructure` method, `ratios.capital_structure` (R1, R2, R3, R6) |
| `src/ratios/liquidity.rs` | `Liquidity` method, `ratios.liquidity` (R1, R2, R3) |
| `src/ratios/working_capital.rs` | `WorkingCapital` method, `ratios.working_capital` (R1, R2, R3) |
| `src/ratios/debt_service.rs` | `DebtService` method, `ratios.debt_service` (R1 to R4, R6) |
| `src/registry.rs` | Five lines in the registry list, in a `ratios` group after `dcf` |

Type paths such as `fairmark_core::ratios::liquidity::Liquidity` satisfy the ADR 0009 test.
The groups do not compose other methods; LLCR reuses `tvm::discount_factor` (SPEC-0002).

### The `Ratio` trait
ADR 0009 introduces a `Ratio` trait with SPEC-0007, its first consumer: formula, numerator,
denominator, balance convention and not-defined handling. Here:

- a `Term` is a named amount already resolved under the balance convention (`equity` at
  closing, or the average of opening and closing, with its own explain step);
- `Ratio` exposes `label`, `formula`, `numerator`, `denominator`, an optional multiplier (the
  day basis) and an optional not-meaningful rule; its provided method `evaluate` writes the
  explain step with the numbers substituted, or returns not defined with the reason (R3);
- `Quotient` (numerator / denominator) and `DayRatio` (numerator / denominator × days)
  implement it. Absolute amounts (structure margins, net working capital, treasury margin) are
  differences, always defined, and are plain explain steps.

### Inputs (how analysts read statements)
Every group takes the flows of the period in `income_statement` (where it needs flows), the
closing balance sheet in `closing` and, for average balances, the opening balance sheet in
`opening`. Each group declares only the aggregates it uses. `balances` is required in every group
(R2); `day_basis` is required in `ratios.working_capital` (R2); `operating_return_base` is
required in `ratios.profitability`; `ebitda_definition` (free text, echoed in the conventions) is
required wherever EBITDA is an input.

The balance convention applies to every balance-sheet amount the group uses, numerator or
denominator, so that a stock is never compared with a flow of a different period and the two
stocks of a stock-to-stock ratio are measured on the same basis. With `closing`, an `opening`
block that is given is not used and the conventions say so. With `average`, a missing `opening`
block is `FM-E001`.

### Net financial position (R6)
`NetFinancialPosition { gross_debt, cash, other_items: [{name, amount}] }`;
NFP = gross debt − cash + Σ other items, positive when the company has net debt (the sign used
by `dcf.fcff` for `net_debt`). Every component is echoed as an explain step; Fairmark adds what
is given and decides nothing about the composition.

### Outputs
A ratio is `RatioValue { value, not_defined }`: `value` is a decimal (fraction, multiple or
days, as the field documents) or null; `not_defined` is null or the reason. Amounts are plain
decimals. The resolved balances and the net financial position are returned too, so the
analyst sees the numbers actually used.

## Schema changes
Additive only: five new methods at version 1 (`schemas/ratios.*/v1.json`), five entries in
`schemas/index.json`. No existing method changes. No new error code: a conditionally missing
block (`opening`, `invested_capital`) is a missing field, `FM-E001`; an empty debt schedule is
an empty cash flow series, `FM-E002`; a discount rate at or below −100% is `FM-E003`;
overflow is `FM-E008`.

## Numerical approach
Each ratio is one checked division (`crate::num`), day-based ratios one multiplication by the
day basis before the division, averages one addition and one division by two. LLCR discounts
each CFADS with `tvm::discount_factor(r, t)` at t = 1..n (end of period), sums with checked
additions and divides by the debt outstanding. No iteration, no rounding beyond the decimal
type's 28 significant digits.

### Definitions implemented
| Group | Output | Definition |
| --- | --- | --- |
| profitability | `roe` | net income / equity; not defined when equity is zero or negative (R3) |
| profitability | `roi` | EBIT / operating return base (invested capital or total assets) |
| profitability | `ros` | EBIT / revenue |
| profitability | `ebitda_margin`, `ebit_margin` | EBITDA / revenue, EBIT / revenue |
| profitability | `capital_turnover` | revenue / operating return base, so that ROI = ROS × capital turnover |
| profitability | `roe_three_factor` | net margin (net income / revenue) × asset turnover (revenue / total assets) × equity multiplier (total assets / equity) |
| capital structure | `debt_to_equity` | gross debt / equity (gross debt from the NFP composition); not defined when equity is not positive |
| capital structure | `nfp_to_equity` | NFP / equity; not defined when equity is not positive |
| capital structure | `equity_ratio` | equity / total assets |
| capital structure | `primary_structure_margin` | equity − fixed assets |
| capital structure | `secondary_structure_margin` | equity + non-current liabilities − fixed assets |
| capital structure | `primary_fixed_asset_coverage`, `secondary_fixed_asset_coverage` | equity / fixed assets; (equity + non-current liabilities) / fixed assets |
| liquidity | `current_ratio`, `quick_ratio`, `cash_ratio` | current assets / current liabilities; (current assets − inventories) / current liabilities; cash / current liabilities |
| liquidity | `net_working_capital`, `treasury_margin` | current assets − current liabilities; current assets − inventories − current liabilities |
| working capital | `days_sales_outstanding` | trade receivables / revenue × days |
| working capital | `days_inventory_outstanding` | inventories / cost of sales × days |
| working capital | `days_payable_outstanding` | trade payables / purchases × days |
| working capital | `cash_conversion_cycle` | DSO + DIO − DPO; not defined when one of them is not |
| debt service | `interest_cover_ebit`, `interest_cover_ebitda` | EBIT / interest, EBITDA / interest |
| debt service | `nfp_to_ebitda` | NFP / EBITDA |
| debt service | `dscr` | CFADS_t / debt service_t, one value per period (R4) |
| debt service | `llcr` | Σ CFADS_t / (1 + r)^t over the schedule / debt outstanding (R4) |

The working capital denominators follow the CFA Institute curriculum (receivables turnover on
revenue, inventory turnover on cost of sales, payables turnover on purchases), which is also
Italian practice (giorni crediti, giorni scorte, giorni fornitori). The three-factor ROE
decomposition is the DuPont form of the same curriculum.

## Test strategy
| Test | Source of expected values | Proves |
| --- | --- | --- |
| Golden `ratios.profitability/closing-balances.yaml`, `average-balances.yaml` | `scripts/reference_ratios.py` (decimal, 50 digits, local context), closed form written out | AC1, AC2 |
| Golden `ratios.profitability/negative-equity.yaml`, `zero-revenue.yaml` | same, reasons by construction | AC3 |
| Golden `ratios.capital_structure/closing-balances.yaml`, `average-balances.yaml`, `negative-equity.yaml` | same | AC1, AC2, AC3 |
| Golden `ratios.liquidity/closing-balances.yaml`, `zero-current-liabilities.yaml` | same | AC1, AC3 |
| Golden `ratios.working_capital/closing-365.yaml`, `average-360.yaml`, `zero-purchases.yaml` | same | AC1, AC2, AC3 |
| Golden `ratios.debt_service/loan-schedule.yaml`, `zero-debt-service.yaml` | same, and the LLCR of a level annuity in closed form | AC1, AC3, AC4 |
| Golden error cases (`average` without `opening`, empty schedule) | constructed | R2 |
| Contract cases `tests/contracts/ratios.*.yaml` | constructed | extreme inputs, conventions |
| Unit tests in `src/ratios/*.rs` | constructed | R3 rules, identities (ROI = ROS × turnover, DuPont product = ROE) |

## Documentation
- Methodology pages `docs/en/methods/ratios-*.md` (five), nav entries, index rows; Italian
  edition through the glossary and `scripts/i18n.py stamp`.
- `docs/en/reference/methods.md` and schemas regenerated. No error code added.
- Python helpers in `python/fairmark/ratios.py` (translate only).
- Release note fragment `changes/114.feature.md`.

## Risks and alternatives
- **Flat inputs rejected.** One flat object per group would hide which amounts are balances
  (subject to the convention) and which are flows; the `closing` and `opening` blocks make it
  visible and let the same block be reused across groups.
- **Average applied only to denominators rejected.** R2 names denominators; in DSO, DIO, DPO
  and NFP / EBITDA the balance is the numerator. Applying the convention to every balance keeps
  each ratio consistent; listed below for confirmation.
- **Stable error codes.** No new code is needed, which also avoids a collision with SPEC-0006
  work in progress.

## Open questions (not implemented until the maintainer decides)
1. **`ratios.covenants` (R5, AC5).** Where the tested value comes from: (a) the covenant names a
   ratio of a group and carries that group's inputs at the test date, and the engine computes
   it; (b) the covenant carries its own numerator and denominator as defined in the loan
   agreement ("each covenant states its own definition"); (c) the value is given and the
   method only tests it. Also: whether a not-defined ratio is a breach, untested or an error,
   and which DSCR (which period) a test date refers to.
2. **ROE decomposition, leverage form.** ROE = [ROI + (ROI − i) × D/E] × (1 − t) needs: D as
   gross debt or NFP (the identity holds with invested capital = equity + NFP); i as interest /
   D on closing or average D; t as taxes / pre-tax income or a statutory rate; and how the
   non-operating effect is reported (the residual between the formula and the actual ROE,
   additive or as a ratio). Inputs for taxes and pre-tax income are not in R1.
3. **ROA.** Net income / total assets (international usage) or operating income / total
   assets (common Italian usage). The second coincides with ROI on a total-assets base.
4. **ROS.** Implemented as EBIT / revenue, equal to the EBIT margin, so that ROI = ROS ×
   capital turnover. Confirm, or name a different numerator (for example operating income from
   core operations) as an additional input.
5. **Fixed-asset coverage.** The spec names one ratio; both the primary (equity / fixed
   assets) and the secondary ((equity + non-current liabilities) / fixed assets) quotients are
   returned. Confirm both, or name one.
6. **Balance convention scope.** Confirm that `average` applies to every balance-sheet amount
   (numerator and denominator), including stock-to-stock ratios of capital structure and
   liquidity.
7. **Not-meaningful cases beyond negative equity.** Whether negative EBITDA (NFP / EBITDA),
   negative interest (interest cover), negative invested capital (ROI) or negative revenue make
   a ratio not defined. Version 1 reports not defined only for zero denominators and for ratios
   over equity when equity is zero or negative.
