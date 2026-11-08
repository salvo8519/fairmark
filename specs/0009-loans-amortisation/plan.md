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
| `src/error.rs` | Four variants `FM-E020` to `FM-E023` |
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

### Error codes
`FM-E018` and `FM-E019` belong to SPEC-0006 (terminal value), merged first. This spec adds:

| Code | Variant | Requirement |
| --- | --- | --- |
| `FM-E020` | `BalloonOutOfRange` | R7, R15, R17 |
| `FM-E021` | `LoanNeverRepaid` | R9 |
| `FM-E022` | `GracePeriodOutOfRange` | R15, R16 |
| `FM-E023` | `RoundingTooCoarse` | R21 |

Error messages name the input field or fields involved and state the valid range or the fix,
for example "`balloon_amount` must be at least 0 and below `principal` (1000); got -1".

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
   `n* = P / A` at `i = 0`; computed as `q × f(−i q) / f(i)` with `q = P' / A` and
   `f(x) = ln(1 + x) / x` summed from its series for |x| < 0.01. `N = ⌈n*⌉` (an `n*` within
   1e-20 of an integer taken as that integer) is confirmed by the exact balance recurrence
   (`b_k = (b_(k−1) − A) (1 + i)`), which also gives the final instalment, so it agrees with
   the schedule (decision 1).
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
- **Logarithm accuracy and the ceiling of `n*`**: `N` from the closed form is confirmed by the
  recurrence with a tolerance of 1e-20 × principal, so a last-digit excess never becomes an
  extra instalment of 1e-20 and a logarithm error never changes `N`.
- **Final adjustment on long terms at high rates**: the instalment is stored with 28
  significant digits; without rounding the final adjustment is that storage error amplified by
  (1 + i)^n: below 1e-18 of the principal while (1 + i)^n <= 1e10, about 4e2 on 150,000 at 5%
  over 1200 periods. This is a property of the stored instalment, not of the recurrence.
- **Rate conversion accuracy**: conversions between different compoundings use series
  functions; golden tolerances on such cases are 1e-19 or looser where stated.
- **Explain size**: one step per period was rejected (decision 8); rows carry the numbers.
- **Final adjustment definition with a balloon**: the balloon is excluded from
  `final_adjustment`, so that without rounding the adjustment is zero up to the arithmetic,
  as AC9 requires, also when a balloon is paid.
- **Error numbering**: final after the rebase on SPEC-0006 (`FM-E018`, `FM-E019` on main).
- **Shared annuity factor**: SPEC-0006 extracted `tvm::annuity_factor` (with the underflow of
  long horizons taken as 0); the loan methods and `tvm.future_value` use that one function.
  The outputs of `tvm.annuity` and `tv.limited_horizon` are byte-identical to `main` on a grid
  of 960 inputs.
- **FM-E003 message**: the message of `RateOutOfRange` is shared by every method and names the
  rate value, not the field (`rate` or `periodic_rate`); changing it is outside this spec.

## Implementation decisions (confirmed by the maintainer, 2026-11-08)
Choices where the spec is silent, taken during the implementation and confirmed by the
maintainer on 2026-11-08 (the library is used mainly by AI agents: one condition, one error
code; explicit, machine-readable conventions; error messages that name the field and the fix).

1. **Whole number of periods.** `loans.number_of_instalments` takes an exact number of periods
   within 1e-20 of an integer as that integer (tightened from 1e-15 after the numerical audit
   of 2026-11-08), then confirms the number of instalments with the exact balance recurrence:
   N is the first instalment whose amount owed exceeds the instalment by at most 1e-20 ×
   principal (corrected down or up from the closed form); the limit of 10000 applies to the
   corrected N. Both are reported in `explain.conventions`. n* is computed with the series of
   ln(1 + x) / x for |x| < 0.01, so small rates keep every digit (about 1e-24 against the
   reference). Confirmed by the maintainer; tolerance and recurrence check after the audit
   (B1: small negative rates gave n + 1 instalments, an internal error or a false limit error).
2. **Final adjustment.** `final_adjustment` excludes the balloon amount (last instalment −
   balloon − regular instalment of the last period); for a fixed amortisation schedule it is
   measured on the principal portion; for bullet it is 0. Confirmed.
3. **Instalment that never repays the loan.** `FM-E021` whatever the sign of the rate: an
   instalment of 0 or below, or not above the interest of the first period. With a negative
   rate and an instalment of 0 or below the balance never reaches zero either. Changed by the
   maintainer from the first implementation (`FM-E001` for a non-positive instalment with a
   negative rate); golden case `error-negative-rate-instalment-zero.yaml`.
4. **Bullet with a balloon amount or a grace period.** Any `balloon_amount` or
   `principal_grace_periods` field given with `bullet` is refused, even 0 (`FM-E020`,
   `FM-E022`): "given" is read literally. Confirmed.
5. **Totals of `loans.instalment`.** `total_paid` = n × instalment as returned (rounded when
   rounding is chosen) + balloon amount; the final-period adjustment belongs to the schedule.
   Confirmed.
6. **Timing in the Python helpers.** `timing` is required in every helper, including
   `amortisation_schedule`, where only `ordinary` is accepted: no adapter default. Confirmed.
