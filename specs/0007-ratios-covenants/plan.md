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

## Covenants (`ratios.covenants`, issue #147, R5, AC5)

Implements decision 1 of 2026-11-08: the tested value comes from the numerator and the
denominator defined by the loan agreement; a ratio that cannot be computed is "not tested" with
the reason, never a breach; the test date refers to the period ending on that date. The method
is read mainly by AI agents, so every outcome is an explicit machine-readable value and every
choice that changes an outcome is a required input.

### Module
`src/ratios/covenants.rs`: `Covenants` method, `ratios.covenants`, one line in the registry
after `ratios::debt_service::DebtService`. It does not call the other ratio groups: the
covenant's own definitions replace the standard ratios.

### Input
| Field | Content |
| --- | --- |
| `covenants` | List of at least one covenant; empty is `FM-E001` |
| `covenants[i].name` | Name of the covenant as in the agreement, not blank (`FM-E001`) |
| `covenants[i].test_date` | ISO 8601 date; the amounts refer to the period ending on it (`FM-E009` when invalid) |
| `covenants[i].numerator`, `.denominator` | `{definition, amount}`: the definition in the words of the agreement (not blank, `FM-E001`) and the amount in currency units |
| `covenants[i].operator` | Required, no default: `at_most` (≤, inclusive maximum), `less_than` (<, strict maximum), `at_least` (≥, inclusive minimum), `greater_than` (>, strict minimum) |
| `covenants[i].threshold` | In the unit of numerator / denominator: a multiple or a decimal fraction (0.30 = 30%) |
| `covenants[i].if_denominator_not_positive` | Required, no default: `breached`, `passed` or `not_tested`, the outcome when the denominator is zero or negative, as the agreement states |
| `covenants[i].rounding` | Optional: `{decimal_places, mode}`, both required when given; `decimal_places` 0 to 6 (`FM-E001` otherwise), `mode` `half_up` (halves away from zero) or `half_even`. Absent: no rounding |

### Output
`covenants`: one result per covenant, in the input order, and `summary` with the number of
covenants `passed`, `breached` and `not_tested`. Each result:

| Field | Content |
| --- | --- |
| `name`, `test_date`, `operator`, `threshold` | Echoed from the input |
| `outcome` | `passed`, `breached` or `not_tested`, decided exactly (see "Precision") |
| `value` | The value compared with the threshold: numerator / denominator to 28 significant digits, or rounded as `rounding` states; null when the denominator is not positive or the covenant is not tested |
| `value_unrounded` | With `rounding`: numerator / denominator before the rounding, to 28 significant digits; otherwise null |
| `headroom` | Distance from the threshold in the unit of the tested value, positive on the compliant side and negative on the breach side: threshold − value for a maximum, value − threshold for a minimum (on the rounded value with `rounding`); null when `value` is null |
| `headroom_relative` | `{value, not_defined}`: headroom / threshold, decimal fraction; not defined when the threshold is zero, when the threshold is negative (open question 3 below) or when the covenant is not tested |
| `not_tested_code` | Null, `denominator_not_positive` or `overflow` (machine-readable) |
| `not_tested_reason` | Null, or why the covenant was not tested, in words |

At the threshold the headroom is zero: an inclusive operator passes, a strict operator breaches.

### Precision
Without `rounding`, the outcome is decided exactly; value and headroom are shown to the 28
significant digits of the decimal type, not rounded to the decimals of the agreement. The
decision is the sign of
numerator − threshold × denominator (denominator positive at that point), computed with
wide-integer arithmetic on the decimal mantissas and scales (a small multiply helper on 64-bit
limbs in `covenants.rs`), so a quotient that rounds to the threshold at 28 digits cannot change
the outcome (numerical audit, finding 1). With `rounding`, the rounded value is computed
exactly from the rational numerator / denominator (wide-integer division of the mantissas,
then the rounding rule on the remainder), and the outcome is the exact comparison of the
rounded value with the threshold.

### Rules
- Zero or negative denominator (including `-0`): the outcome is `if_denominator_not_positive`;
  value and headroom are null; when the outcome is `not_tested` the code is
  `denominator_not_positive` and the reason says "denominator is zero: <definition>" or
  "denominator is negative (<amount>): <definition>".
- Not tested (never a breach), the others continue: the tested value, the rounded value or the
  headroom exceeds the range of the decimal type, code `overflow`. A relative headroom
  that overflows is not defined, with the reason; the outcome stands.
- A negative numerator is compared as computed (a net cash position gives a negative leverage
  that passes a maximum; a negative EBITDA gives a negative cover that breaches a minimum).
- No error code is added: missing, blank or unknown fields are `FM-E001`, invalid dates
  `FM-E009`; overflow of one covenant is `not_tested`, not an error.

### Explain
Covenants are numbered from 1 and every label starts with "covenant k, <name> (<test date>)".
Per covenant: numerator and denominator as given, with the definitions; the tested value with
the numbers substituted; with `rounding`, the rounded value next to the unrounded one; the
headroom and the relative headroom; a verdict step ("breached
(600 / 175 at_least 4 is false, decided exactly)"). Then the three summary counts as steps.
Conventions: the definitions come from the agreement, the period ending on the test date, the
precision rule above, the headroom sign convention, "a negative numerator is compared as
computed", the not-tested rule, and for each covenant its test rule ("passes when the tested
value ≤ 3.5"), its rule for a denominator that is not positive (from the input), its rounding
rule (or that the value is not rounded) and, when not tested, the reason.

### Tests
| Test | Source of expected values | Proves |
| --- | --- | --- |
| Golden `ratios.covenants/one-breach.yaml` | `scripts/reference_ratios.py`, `covenants` | AC5 |
| Golden `ratios.covenants/zero-denominator.yaml` | same | decision 1, not tested is never a breach |
| Golden `ratios.covenants/negative-denominator.yaml` | same | `if_denominator_not_positive`, all three values |
| Golden `ratios.covenants/rounding.yaml`, `rounding-modes.yaml` | same, Python `quantize` | contractual rounding (3.504 at most 3.50) |
| Golden `ratios.covenants/at-the-threshold.yaml` | same | inclusive and strict operators |
| Golden `ratios.covenants/overflow.yaml` | constructed | overflow is not tested, the others continue |
| Golden `ratios.covenants/negative-numerator.yaml` | same | sign of the tested value |
| Golden `ratios.covenants/non-positive-threshold.yaml` | same | relative headroom not defined |
| Golden error cases (empty list, blank definition, missing operator, missing `if_denominator_not_positive`, decimal places above 6, invalid date) | constructed | errors |
| Contract cases `tests/contracts/ratios.covenants.yaml` | constructed | extreme inputs, conventions |
| Unit tests in `src/ratios/covenants.rs` | constructed | rules above; exact outcome on values that round to the threshold at 28 digits |

### Open questions, resolved (maintainer, 2026-11-08, covenant tests)
Decided on 2026-11-08 and recorded in `spec.md` ("Decisions (maintainer, 2026-11-08, covenant
tests)"); implemented in version 1 before the first merge.

1. **Negative or zero denominator.** Resolved: required input `if_denominator_not_positive`
   (`breached`, `passed`, `not_tested`). The `FM-E001` refusal of a negative denominator is
   removed; value and headroom are null; `not_tested_code` is `denominator_not_positive`.
2. **Rounding of the tested value.** Resolved: optional input `rounding` (`decimal_places` 0 to
   6, `mode` `half_up` or `half_even`). The rounding is exact on the rational value
   numerator / denominator (wide-integer division, so no double rounding through the
   28-digit quotient); the outcome is decided exactly on the rounded value; `value` is the
   value compared and `value_unrounded` the value before rounding.
3. **Relative headroom on a negative threshold.** Still not defined, with the reason (not part
   of the 2026-11-08 decisions).
4. **Covenants on a single amount.** Out of scope; a separate roadmap task.
5. **Equity cure and step-downs.** Equity cure out of scope in version 1; a step-down is one
   covenant per test date with its own threshold.
6. **Negative numerator.** Confirmed: compared as computed.
7. **Strict operators.** Confirmed as an extension of R5.
