# SPEC-0007 — Plan

Status: active (SPEC-0007 approved on 2026-11-07); changes to the spec are reflected here first.
Issue: #114. The open questions of the first draft were decided by the maintainer on
2026-11-08 (see "Open questions, resolved"); covenants are a separate method and issue.

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
required in `ratios.profitability`; `ebitda_definition` and `interest_definition` (free text,
not empty, echoed in the conventions) are required wherever EBITDA or interest is an input; a
blank text is `FM-E001`.

The balance convention applies to every balance-sheet amount the group uses, numerator or
denominator, so that a stock is never compared with a flow of a different period and the two
stocks of a stock-to-stock ratio are measured on the same basis. With `closing`, an `opening`
block that is given is not used and the conventions say so. With `average`, a missing `opening`
block is `FM-E001`. An `invested_capital` given with a total-assets base is reported as not used.

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
| profitability | `roi` | EBIT / operating return base (invested capital or total assets); not defined when invested capital ≤ 0 |
| profitability | `roa` | EBIT / total assets |
| profitability | `ros` | EBIT / revenue |
| profitability | `ebitda_margin`, `ebit_margin` | EBITDA / revenue, EBIT / revenue |
| profitability | `capital_turnover` | revenue / operating return base, so that ROI = ROS × capital turnover |
| profitability | `roe_leverage` | [ROI + (ROI − i) × D/E] × (1 − t), i = financial charges / D, t = taxes / pre-tax income; optional `leverage` block |
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
| debt service | `interest_cover_ebit`, `interest_cover_ebitda` | EBIT / interest, EBITDA / interest; not defined when interest or the numerator ≤ 0 |
| debt service | `nfp_to_ebitda` | NFP / EBITDA; not defined when EBITDA ≤ 0 |
| debt service | `dscr` | CFADS_t / debt service_t, one value per period (R4); not defined when debt service ≤ 0; empty without a schedule |
| debt service | `llcr` | (Σ CFADS_t / (1 + r)^t + reserve balance) / debt outstanding (R4); not defined when debt outstanding ≤ 0 or without a schedule |

The working capital denominators follow the CFA Institute curriculum (receivables turnover on
revenue, inventory turnover on cost of sales, payables turnover on purchases), which is also
Italian practice (giorni crediti, giorni scorte, giorni fornitori). The three-factor ROE
decomposition is the DuPont form of the same curriculum.

## Test strategy
| Test | Source of expected values | Proves |
| --- | --- | --- |
| Golden `ratios.profitability/closing-balances.yaml`, `average-balances.yaml`, `average-invested-capital.yaml` | `scripts/reference_ratios.py` (decimal, 50 digits, local context), closed form written out | AC1, AC2 |
| Golden `ratios.profitability/negative-equity.yaml`, `zero-revenue.yaml` | same, reasons by construction | AC3 |
| Golden `ratios.capital_structure/closing-balances.yaml`, `average-balances.yaml`, `negative-equity.yaml` | same | AC1, AC2, AC3 |
| Golden `ratios.liquidity/closing-balances.yaml`, `average-balances.yaml`, `zero-current-liabilities.yaml` | same | AC1, AC2, AC3 |
| Golden `ratios.working_capital/closing-365.yaml`, `closing-360.yaml`, `average-360.yaml`, `zero-purchases.yaml` | same | AC1, AC2, AC3 |
| Golden `ratios.debt_service/loan-schedule.yaml`, `zero-denominators.yaml`, `zero-debt-outstanding.yaml` | same, and the LLCR of a level annuity in closed form | AC1, AC3, AC4 |
| Golden error cases (`average` without `opening`, blank definitions, empty schedule) | constructed | R2 |
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

## Open questions, resolved (maintainer, 2026-11-08)
All questions below were decided on 2026-11-08; the decisions are recorded in `spec.md`
("Decisions (maintainer, 2026-11-08)") and implemented in version 1, before the first merge.

1. **`ratios.covenants` (R5, AC5).** Resolved, not in this issue: a separate method and issue.
   The tested value comes from the numerator and denominator defined by the loan agreement; a
   ratio that cannot be computed is "not tested" with the reason, never a breach; the test date
   refers to the period ending on that date.
2. **ROE decomposition, leverage form.** Resolved and implemented as `roe_leverage`:
   `leverage.debt_basis` is required (`gross_financial_debt` or `net_financial_position`, no
   default); i = financial charges / D; t = taxes / pre-tax income. D is the `debt` field of
   the balance sheets, so the balance convention applies to it like every other balance;
   financial charges, taxes and pre-tax income are in the optional `leverage` block. Without
   the block the form is not defined ("leverage inputs not given"). No residual is reported:
   the form is shown next to ROE, and the page explains that they coincide when invested
   capital equals equity + D.
3. **ROA.** Resolved: EBIT / total assets, implemented as `roa`.
4. **ROS.** Confirmed: EBIT / revenue.
5. **Fixed-asset coverage.** Confirmed: primary and secondary, both.
6. **Balance convention scope.** Confirmed: every balance-sheet amount, numerator included.
7. **Not-meaningful cases.** Resolved: NFP / EBITDA when EBITDA ≤ 0; an interest cover when
   interest or its numerator (EBIT or EBITDA) ≤ 0; ROI when invested capital ≤ 0 (invested
   capital base); DSCR when debt service ≤ 0; LLCR when debt outstanding ≤ 0. Implemented in
   the `Ratio` trait as a positive-denominator and a positive-numerator rule, with a golden case
   for each. Negative revenue stays computed (not in the decision).
8. **Debt schedule optional.** Resolved: `debt_schedule` is optional; without it `dscr` is an
   empty list, `pv_cfads` is null and `llcr` is not defined ("no debt schedule given").
9. **Golden tolerance.** Resolved: 1e-20 for every `ratios.*` golden case.
10. **Negative EBITDA, interest or debt service.** Resolved with question 7.
11. **LLCR reserve account.** Resolved: optional `debt_schedule.reserve_balance`, added
    undiscounted at the valuation date to the LLCR numerator; the discount rate is documented as
    the cost of debt per period chosen by the analyst.
