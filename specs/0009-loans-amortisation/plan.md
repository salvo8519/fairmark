# SPEC-0009 — Plan

Status: active (SPEC-0009 approved on 2026-11-08); changes to the spec are reflected here first.
Two phases on one branch: phase A (issue #136: `loans.instalment`,
`loans.number_of_instalments`, `tvm.future_value`) and phase B (issue #137:
`loans.amortisation_schedule`).

## Design

### Module layout in `crates/fairmark-core`
| File | Content |
| --- | --- |
| `src/loans.rs` (new, theme `loans`) | `PaymentFrequency`, `Rounding`, `RoundingMode`, `AmortisationType`; the shared rate-form helper (R1, R1a, R5); `instalment`, `number_of_instalments`, `amortisation_schedule` with their input and output types; unit types `Instalment`, `NumberOfInstalments`, `AmortisationSchedule` implementing `Method` |
| `src/tvm.rs` | `tvm.future_value` (`FutureValueInput`, `FutureValueOutput`, unit type `FutureValue`); the annuity factor of `tvm.annuity` extracted into `pub(crate) fn annuity_factor`, so that the loan methods and the future value reuse it (no change to the numbers or the explain of `tvm.annuity`) |
| `src/rates.rs` | Unchanged; `rates::convert` is called by the rate-form helper |
| `src/error.rs` | Four variants `FM-E020` to `FM-E023` (provisional numbering, see below) |
| `src/registry.rs` | One line per new method: `tvm::FutureValue` in the `tvm` group, the three `loans::*` lines in a new `loans` group after `tvm` |

Type paths `fairmark_core::loans::Instalment` and `fairmark_core::tvm::FutureValue` satisfy the
ADR 0009 test (the path starts with the theme module).

### Reuse, no duplication
- **Periodic rate (R1)**: form (a) calls `rates::convert` with `from` = the given compounding
  and `to` = the compounding equal to the payment frequency, then divides by the payments per
  year. The steps of `rates.convert` are copied into the loan explain, followed by the
  division step. Form (b) uses the rate as given.
- **Annuity factor (R6)**: the instalment is `(P − B v^n) / a`, where `a` is the factor of
  `tvm.annuity` with payment 1, growth 0 and the same timing; `v^n` is
  `tvm::discount_factor`. The near-zero-rate branch of the annuity factor (direct sum when
  |i| < 1e-9) is inherited. Rate exactly zero uses the limit form of R4 as its own step.
- **Future value (R10)**: the payment term is `PMT × a × (1 + r)^n`, the same annuity factor
  accumulated to t = n; `(1 + r)^n` is `rates::pow` (exact repeated multiplication).
- **Schedule (R13)**: the French instalment calls the same function as `loans.instalment`
  (ordinary timing, `n − g` periods, balloon `B`).

### Shared rate inputs
Each loan input carries `rate`, `compounding`, `payment_frequency` (form a) and
`periodic_rate` (form b) as optional fields; one helper validates that exactly one complete form
is given (`FM-E001` naming the missing or conflicting fields, R1a), checks that the periodic
rate exceeds −1 (`FM-E003`) and reports the conventions and steps of R5. `serde(flatten)` is
not used because it does not combine with `deny_unknown_fields`.

### Error codes (provisional)
`main` ends at `FM-E017`; `FM-E018` and `FM-E019` are taken by SPEC-0006 on an unmerged
branch. This spec uses:

| Code | Variant | Requirement |
| --- | --- | --- |
| `FM-E020` | `BalloonOutOfRange` | R7, R15, R17 |
| `FM-E021` | `LoanNeverRepaid` | R9 |
| `FM-E022` | `GracePeriodOutOfRange` | R15, R16 |
| `FM-E023` | `RoundingTooCoarse` | R21 |

The numbers become final when the merge order with SPEC-0006 is settled; until then they are
documented in `docs/en/reference/errors.md` as on any other code.

## Schema changes
All additive: four new methods at version 1, snapshots `schemas/<id>/v1.json`, entries in
`schemas/index.json` with `introduced_in: unreleased`. No existing method changes version:
the extraction of the annuity factor is a refactoring with identical numbers and explain
(the golden cases of `tvm.annuity` prove it).

| Method | Required | Optional (default) |
| --- | --- | --- |
| `loans.instalment` | `principal`, `number_of_instalments`, `timing`, one rate form | `balloon_amount` (0), `rounding` (none) |
| `loans.number_of_instalments` | `principal`, `instalment`, `timing`, one rate form | none |
| `tvm.future_value` | `rate`, `periods`, `timing`, at least one of `present_value`, `payment` | `present_value` (0), `payment` (0) |
| `loans.amortisation_schedule` | `principal`, `number_of_instalments`, `timing` (`ordinary` only), `amortisation_type`, one rate form | `principal_grace_periods` (0), `balloon_amount` (0), `rounding` (none) |

`rounding` is an object with two required fields, `decimal_places` (0 to 6) and `mode`
(`half_up`, `half_even`): a rounding without a mode is a missing field, `FM-E001` (AC9).

## Numerical approach
1. **Periodic rate**: form (a), `j = rates::equivalent_rate(rate, compounding, frequency)`,
   then `i = j / p`. When the compounding equals the payment frequency the conversion is the
   identity and `i = rate / p` exactly; otherwise it uses the `ln` and `exp` series of
   `rust_decimal` (relative error around 1e-26, verified by the `rates.convert` golden cases).
2. **Instalment**: `A = (P − B v^n) / a_n` (ordinary) or `/ ä_n` (due); `v^n` exact up to one
   division; limit `(P − B) / n` at `i = 0`. Rounding, when requested, is applied once to `A`
   with `round_dp_with_strategy` (`half_up`: halves away from zero; `half_even`: halves to the
   even digit). Totals: `n × A + B` with the instalment as returned, and `total_paid − P`.
3. **Number of instalments**: `n* = −ln(1 − i P' / A) / ln(1 + i)`, `P' = P` or `P / (1 + i)`;
   `n* = P / A` at `i = 0`. `n*` uses the logarithm series (approximate, about 1e-25 relative);
   to keep `⌈n*⌉` stable, an `n*` within 1e-15 of an integer is taken as that integer, reported
   as a convention (the loan is then repaid by that whole number of instalments up to the
   digits of the given instalment). The final instalment follows from the exact recurrence of
   the balance over `N − 1` periods (`b_k = b_(k−1) (1 + i) − A`, or its in-advance form),
   so it agrees with the schedule.
4. **Future value**: `FV = PV (1 + r)^n + PMT × a × (1 + r)^n` with `a` the ordinary or due
   factor; `PV + n × PMT` at `r = 0`.
5. **Schedule**: one pass over the periods with checked arithmetic. Grace periods: interest
   only. French: constant (rounded) instalment, principal = instalment − interest. Fixed
   amortisation schedule: constant (rounded) principal `(P − B) / (n − g)`, instalment =
   principal + interest. Bullet: interest only, the principal with the last instalment. Last
   period: principal = opening balance (balloon included), `final_adjustment` = last
   instalment − balloon − regular instalment of that period (for the fixed schedule, regular
   principal + interest of the last period; for bullet it is 0). With rounding, a balance
   below zero before the last period, or a negative last principal or instalment, is
   `FM-E023`. Interest is never rounded (R19).
6. **Explain size (R22)**: a fixed number of steps independent of `n`: periodic rate, regular
   amount (and its rounding), the recurrence stated once, the first period (and, with a grace
   period, the first amortising period) with numbers, the last period and the final
   adjustment, the three totals. The rows carry every per-period number.

## Test strategy
- **Reference**: `scripts/reference_loans.py`, independent of the Rust code, 50 significant
  digits in a local `decimal` context (pattern of `scripts/reference_analysis.py`): periodic
  rate through `exp`/`ln` of the annual growth, closed-form instalment written from R6
  (not via an annuity factor), `n*` by `ln`, final instalment and schedules by the period
  recurrence, future value from R10 with the direct `((1 + r)^n − 1) / r` form.
- **Golden cases** (`tests/golden/<id>/`), each with its source:
  - ECMA-376 worked examples (AC1, AC4, AC5), tolerance 0.005 because they are published to
    two decimals;
  - the same loans against the 50-digit reference with the tightest tolerance the values
    allow (1e-20 for amounts up to 1e6; 0 for exact values such as zero-rate cases and
    fixed-principal schedules with terminating decimals);
  - textbook worked examples (AC15): Watts, *Business and Financial Mathematics*, section 6.1
    (50,000 business loan, 5% compounded quarterly, semi-annual payments, rounded to the
    cent) and Olivier, *Business Math: A Step-by-Step Handbook*, Example 13.3A (dishwasher
    loan), with edition and page in the file; tolerance 0.005 because the books print cents;
  - error cases with `expect_error` for every new code and for the `FM-E001` cases of AC12.
- **Contract cases** `tests/contracts/<id>.yaml`: extreme inputs (MAX, MIN) and the
  conventions of R5, R11, R19.
- **Rust unit tests** in `loans.rs`: AC12a (form a versus the equivalent form b), AC13
  (bounded explain), AC14 properties on a grid of loans (sum of principal portions, sum of
  instalments, non-increasing balances, present value of the instalments equals the
  principal within 1e-18 without rounding), AC8 (time k), AC10.
- **Python**: `python/tests/test_reference_loans.py` compares the engine with the reference
  on seeded random loans (instalments, schedules); `python/tests/test_cross_check.py` gains
  `numpy_financial` checks (`pmt`, `nper` on ten random loans, `fv`, `ipmt`, `ppmt`) at the
  usual relative tolerance 1e-9.

## Documentation
- Methodology pages `docs/en/methods/loans-instalment.md`,
  `loans-number-of-instalments.md`, `loans-amortisation-schedule.md`, `tvm-future-value.md`,
  with runnable examples; nav entries in `zensical.toml`; rows in `docs/en/methods/index.md`.
- `docs/en/reference/errors.md` rows for `FM-E020` to `FM-E023`.
- Italian edition of every changed page with `i18n/glossary.yaml` (new terms added to the
  glossary) and the hash stamps of `scripts/i18n.py`.
- `uv run just generate` for the schema snapshots and the methods reference.
- Release note fragments `changes/136.feature.md` and `changes/137.feature.md`.
- The Banca d'Italia quotations of the spec are not reproduced in the methodology pages; the
  pages cite only the regulatory definitions of the amortisation types.

## Risks and alternatives
- **Logarithm accuracy and the ceiling of `n*`**: a tolerance on the integer snap is the
  alternative to computing `N` only from the recurrence; the recurrence alone would turn a
  last-digit excess into an extra instalment of 1e-20. The snap (1e-15) is reported.
- **Rate conversion accuracy**: conversions between different compoundings use series
  functions; golden tolerances on such cases are 1e-19 or looser where stated.
- **Explain size**: one step per period was rejected (decision 8); rows carry the numbers.
- **Final adjustment definition with a balloon**: the balloon is excluded from
  `final_adjustment`, so that without rounding the adjustment is zero up to the arithmetic,
  as AC9 requires, also when a balloon is paid.
- **Error numbering**: may need renumbering after SPEC-0006 merges; only `error.rs`, the
  error reference and the error golden cases would change, before any release.
