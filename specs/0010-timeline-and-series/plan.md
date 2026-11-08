# SPEC-0010 — Plan

## Design

### Shared frequency (R20, R21)
- **New module** `crates/fairmark-core/src/frequency.rs` (no methods, like `num.rs`), declared in
  `lib.rs`. It holds the one definition of the named frequencies:
  `pub enum Frequency { Annual, SemiAnnual, FourMonthly, Quarterly, Bimonthly, Monthly }` with
  `serde(rename_all = "snake_case")`, `JsonSchema`, a doc comment per value stating its periods
  per year and months ("every two months, 6 periods a year" for `bimonthly`), and
  `const fn per_year(self) -> u32` and `const fn months(self) -> u32`.
- **Period length** (R3): `pub enum PeriodLength { Named(Frequency), Months(u32) }`, an internal
  type built by validation, never deserialised. `months()`, `per_year() -> Option<u32>` (only
  when the months divide 12), `years() -> Option<u32>` (only when the months are a multiple of
  12) and `describe()` for `explain`. `period_months` equal to a named tenor gives the same
  `PeriodLength` behaviour, so equal results follow by construction (AC2, AC9).
- **`PeriodLengthFields`**: a helper that takes `Option<Frequency>` and `Option<u32>` and the
  JSON pointer prefix, applies the exactly-one rule (or the at-most-one rule of the `periods`
  timeline) and returns `PeriodLength` or `FM-E001` naming both pointers with the fix. It is the
  same pattern as the rate form of `loans` (`RateForm`).
- **`Compounding`** (`rates.rs`) gains `Bimonthly` and `FourMonthly`, appended after the
  existing values so that the regenerated enumeration lists only add entries. A new
  `fn frequency(self) -> Option<Frequency>` maps every periodic value; `per_year()` becomes
  `self.frequency().map(Frequency::per_year)`. `label()` gains the two labels.
- **`PaymentFrequency`** (`loans.rs`) gains the same two values, appended; `compounding()` and
  `name()` are extended, `per_year()` goes through `Frequency`. The Rust type names stay, so the
  `$defs` names of the existing schemas do not change.
- A unit test enumerates `Compounding`, `PaymentFrequency` and `Frequency` and checks that every
  periodic value agrees on its periods per year, so the three lists cannot drift.
- **Timeline domain type (#19, ADR 0009 §5)**: `Frequency` and `PeriodLength` are the first
  pieces of it; this plan does not introduce the full timeline type. Issue #19 can build on
  them without a contract change.

### dates.schedule (R1 to R8), in `dates.rs`
- `ScheduleInput { start_date: Date, frequency: Option<Frequency>, period_months: Option<u32>,
  number_of_periods: u32, day_count: Option<DayCount> }`, `deny_unknown_fields`, doc comment on
  every field. The doc comments of the period fields, and the method `DESCRIPTION`, state the
  name-to-tenor mapping and the [start, end) definition with the literal phrases "from and
  including" and "to but excluding" (AC5). No `oneOf` in the schema: the exactly-one rule is
  stated in the descriptions and enforced in the core, so the portable tool schemas (SPEC-0002
  T10) are unaffected.
- `ScheduleOutput { start_dates, end_dates, year_fractions: Option<Vec<Decimal>>,
  months_per_period: u32, periods_per_year: Option<u32> }`, optional fields skipped when absent.
- `fn add_months(start: Date, months: i64, end_of_month: bool) -> Result<Date>`: month index
  `year × 12 + (month − 1) + months` in `i64` (no overflow for `u32` × 10000), year and month
  from it, day = last day of the month when `end_of_month`, otherwise
  `min(start.day, days_in_month)`; `Date::new` refuses years beyond 2199 and the caller maps it
  to `FM-E009` naming `/number_of_periods` (R7).
- `pub(crate) fn build_schedule(input: &ScheduleInput) -> Result<Schedule>`: validation in the
  order start date, period length, number of periods, then generation; returns the dates, the
  `PeriodLength`, the flags "start is a month end" and "the rule changed a date" (a date whose
  day differs from the start day), and the year fractions through `DayCount::year_fraction`
  (no second implementation). `series.project` calls it (R14).
- `pub fn schedule(input) -> Result<(ScheduleOutput, Explain)>` adds the `explain` of R8; a
  helper `schedule_explain(&Schedule, &mut Explain)` is shared with `series.project`.
- `struct Schedule;` implementing `Method` (`ID = "dates.schedule"`, `VERSION = "1"`), one line
  in the registry list.

### series.project (R9 to R19), new theme `series.rs`
- Input `ProjectInput { base_value, base_period: BasePeriod, growth: GrowthRule,
  growth_basis: Option<GrowthBasis>, timeline: Timeline, timing: Option<ValueTiming> }`.
  `GrowthRule` externally tagged (`constant`, `linear`, `per_period`) as the terminal technique
  of `dcf.fcff`; `Timeline` externally tagged: `periods { number_of_periods, frequency?,
  period_months? }` or `schedule(ScheduleInput)`.
- Steps, in order: resolve the timeline (period length with the at-most-one rule, or
  `build_schedule`); validate the timing against the timeline (R15); count the growth steps s
  (n or n − 1, R10); expand the rule into s stated rates (linear by j / s, R11; count check,
  R12); check the stated rates against −1 (R17); resolve the basis (R13); convert each rate;
  check the converted rates against −1; run the recurrence; build rows, `values` and
  `dated_values`; build `explain`.
- **Basis resolution** (R13), with p the months per period:
  - no period length: `per_period` or absent; an annual basis is `FM-E001` naming
    `/growth_basis`;
  - p = 12: any basis or none (all coincide), reported;
  - p divides 12, p ≠ 12: basis required; `annual_compound` through
    `rates::equivalent_rate(g, Compounding::Annual, c) / m`, with `c` the `Compounding` of the
    shared `Frequency` of tenor p (this is why the two new compounding values are needed for
    p = 2 and p = 4); `annual_simple` = g / m;
  - p multiple of 12, p ≠ 12: basis required; `annual_compound` = (1 + g)^k − 1 with
    `checked_powi` (exact); `annual_simple` = g × k;
  - other p: basis required; only `per_period`; an annual basis is `FM-E001` naming
    `/growth_basis` with the fix.
- Implied effective annual growth (R16), only when p divides 12: (1 + g_p)^m − 1 by integer
  power.
- `dated_values` elements reuse the dated-flow type of `tvm` (the element of `cash_flows` in
  `tvm.xnpv`), so the contract test of AC10 is a type identity, not a copy of fields.
- `struct Project;` implementing `Method` (`ID = "series.project"`, `VERSION = "1"`); `series`
  added to the module list of `lib.rs` and the registry import.

### Reuse
`Date`, `days_in_month`, `DayCount::year_fraction` (`dates`), `equivalent_rate` and
`Compounding` (`rates`), `num::*` checked arithmetic, the dated-flow type of `tvm`, `Explain`
and the registry. No new error code; errors use `FairmarkError::InvalidInput`, `InvalidDate`,
`RateOutOfRange` and `Overflow` with the JSON pointers of ADR 0013.

## Schema changes
- **New, additive**: `schemas/dates.schedule/v1.json`, `schemas/series.project/v1.json`, new
  entries in `schemas/index.json`.
- **Extended current versions, additive** (R22): `rates.convert`, `tvm.xnpv`, `tvm.xirr`,
  `loans.instalment`, `loans.number_of_instalments`, `loans.amortisation_schedule` and every
  other method whose schema embeds `Compounding` or `PaymentFrequency` (found by regenerating
  and reading the diff): two added enumeration values and their descriptions, nothing else. No
  `method_version` bump. If the schema check of CI classifies the change as non-additive, the
  implementation stops at that task and the maintainer decides (Decision 5).
- No default is introduced (R23); `meta.defaults_applied` (#197) is untouched.

## Numerical approach
- **Dates**: integer arithmetic only (month index in `i64`, day clamping); no `Decimal`.
- **Year fractions**: `DayCount::year_fraction` as is; sum with `num::add`.
- **Recurrence**: V_k = V_{k−1} × (1 + g_k) with `num::mul` and `num::add`, period by period, no
  closed-form power (R9). Exact for decimal inputs until the 28 significant digits of
  `rust_decimal` are used up; the methodology page states it.
- **Linear rates**: g_from + (g_to − g_from) × j / s; the division by s is the only inexact
  operation (exact when s divides the difference in the decimal sense).
- **`annual_compound`, p divides 12**: the `ln`/`exp` series of `equivalent_rate` (relative
  error near 1e-27, as stated on `rates-convert.md`); AC9 requires V_12 within 1e-20 relative of
  1120. **p multiple of 12**: integer power, exact. **`annual_simple`**: one division or one
  multiplication.
- **Overflow**: every operation checked; an overflow is `FM-E008` (R17).
- No iterative algorithm; nothing to report under constitution I beyond the fixed rules.

## Test strategy
- **Golden cases** (`tests/golden/dates.schedule/`, `tests/golden/series.project/`, plus new
  files only in `tests/golden/rates.convert/`, `tests/golden/tvm.xnpv/`,
  `tests/golden/loans.instalment/`), each with its `source`:
  - schedules (AC1, AC3, AC4): QuantLib-Python and python-dateutil, both pinned in the
    `oracles` dependency group, the exact versions recorded in every golden file; the
    generator `scripts/reference_schedule.py` builds each schedule with both libraries and
    fails when they disagree. QuantLib builds the `Schedule` from the effective date, a
    termination date equal to the last generated date, `Period(p, Months)`, `NullCalendar`,
    `Unadjusted`, `DateGeneration.Forward`, `endOfMonth = True`;
  - series (AC6 to AC10, AC12): the 50-digit reference `scripts/reference_series.py`
    (Python `decimal`, 50 digits, written from the spec, not from the Rust code), plus the
    published Damodaran rates of AC7 and the hand values of AC6 and AC9;
  - shared frequencies (AC14): the 50-digit reference of SPEC-0002 extended to m = 6 and
    m = 3 (`scripts/reference_time.py`), and `scripts/reference_loans.py` for the bimonthly
    instalment.
- **Property tests** (proptest): AC2 (structure, no drift, named equals months), AC11
  (`constant` equals repeated `per_period`; `linear` with equal ends equals `constant`; equality
  with `dcf.fcff` stages), AC14 round-trips over the seven compounding values.
- **Oracle tests** (Python, `oracles` group, CI matrix as SPEC-0002 T14): random schedules
  against python-dateutil and QuantLib (AC2 last clause, AC3).
- **Unit tests**: descriptions and `explain` content (AC5, AC13), bounded `explain` steps for 4
  and 400 periods, every error with its pointer and fix (AC4, AC8, AC9, AC12), `period_months`
  at `u32::MAX` (AC4), frequency mapping agreement (R20).
- **Regression** (AC15): the whole existing golden suite unchanged; no existing golden file
  edited.

## Documentation
- New pages `docs/en/methods/dates-schedule.md` and `docs/en/methods/series-project.md`
  (content listed in the spec, "Documentation"), nav in `zensical.toml`, rows in
  `docs/en/methods/index.md`, titles in `i18n/method_titles/it.yaml`, glossary terms in
  `i18n/glossary.yaml` (periodo di calcolo, piano dei periodi, durata del periodo in mesi,
  bimestrale, quadrimestrale, regola di fine mese, tasso proporzionale).
- Updated pages for the two new frequencies: `rates-convert.md`, `tvm-xnpv.md`, `tvm-xirr.md`,
  `loans-instalment.md`, `loans-number-of-instalments.md`,
  `loans-amortisation-schedule.md`; `docs/en/reference/errors.md` (new `FM-E001` and `FM-E009`
  cases); ADR 0008 amendment note on §4.
- Generated: schemas, `docs/en/reference/methods.md` (`uv run just generate`).
- `skills/fairmark/SKILL.md`; Italian edition by the `translator`; release-note fragment for
  #206.

## Risks and alternatives
- **QuantLib end-of-month behaviour differs across releases** (spec, "Sources to verify"): the
  version is pinned and recorded; a disagreement between QuantLib and python-dateutil stops the
  generator and is reported to the maintainer, not resolved by picking one.
- **Schema check and enumeration extension** (R22): if CI calls it non-additive, stop and ask.
- **Golden protection hook**: the shared-frequency golden cases are new files in existing
  method folders; if the hook blocks the creation, stop and ask the maintainer (never bypass
  it).
- **`period_months` without an upper bound below `u32::MAX`**: chosen by Decision F; the
  month arithmetic in `i64` and the date range make every value safe (AC4). Alternative
  rejected: a cap of 12 or 120 months, which refuses tenors for no numerical reason.
- **Annual bases for p that neither divides 12 nor is a multiple of 12**: refused (Decision F).
  Alternative rejected for version 1: a fractional power (1 + g)^(p/12) − 1, approximate and
  without a settled convention for the simple basis; it can be added later as an additive
  change if users ask.
- **A single `period_length` object** (`{"frequency": ...}` or `{"months": n}`) was considered
  instead of two sibling fields: rejected because two flat fields map to how analysts and
  QuantLib state it and keep the `periods` timeline flat.
- **`oneOf` in the JSON Schema** for the exactly-one rule: rejected to keep the portable tool
  schemas valid for every provider; the rule is in the descriptions and the core.
