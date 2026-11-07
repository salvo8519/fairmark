# ADR 0008 — Time axis, calendar dates, day counts and rate conventions

Status: accepted · 2026-11-07

## Context
Every valuation method discounts amounts that occur at some point in time. Analysts work both
with periodic plans (year 1 to year 5 of a business plan) and with dated flows (a closing on
30 June, a dividend on 15 May). Practice differs on day counts, compounding and the timing of
flows within a period, and each of these choices changes the result. Without one shared time
model, every method would re-implement discounting with slightly different conventions.

## Decision
1. **One time axis.** Inside the engine every cash flow has a time `t` in years from the
   valuation date. Periodic inputs map to `t = k / periods_per_year` (shifted by the timing
   convention); dated inputs map to `t = year_fraction(valuation_date, date, day_count)`.
   All discounting goes through the functions of the `rates` and `tvm` modules; methods never
   compute discount factors themselves.
2. **Calendar dates are a value type of the core.** `Date` is a proleptic Gregorian calendar
   date without time or time zone, serialised as ISO 8601 `YYYY-MM-DD`, valid from 1900-01-01
   to 2199-12-31. It is implemented in the core (about one hundred lines, civil-from-days
   algorithm) instead of a date crate: the core needs no time of day, no time zones, no clock
   and no parsing beyond ISO 8601, and keeps zero I/O and minimal dependencies (ADR 0001).
3. **Day counts follow the 2006 ISDA Definitions, section 4.16**: Actual/365 (Fixed),
   Actual/360, Actual/Actual (ISDA), 30/360 (Bond Basis) and 30E/360 (Eurobond Basis). Others
   are added only with a published definition.
4. **Compounding is explicit**: annual (effective), semi-annual, quarterly, monthly, or
   continuous. A rate is always given together with its compounding; conversion is a method of
   its own.
5. **Rates may vary over time.** Periodic discounting accepts either one rate or one rate per
   period (a term structure of forward period rates); the discount factor of period k is the
   product of the period factors up to k.
6. **Defaults exist only where a published market standard defines one** (XNPV and XIRR:
   Actual/365 (Fixed), annual compounding, valuation at the first flow, as in ECMA-376). They
   are documented in the schema and reported in `explain.conventions` on every result.
7. **Rate solvers are deterministic and refuse ambiguity.** IRR-type solvers scan a fixed grid
   of rates; if more than one rate sets the present value to zero, they return an error that
   lists the brackets instead of choosing one.

## Consequences
+ Every method shares the same, tested time model; conventions are visible in every result.
+ Dated and periodic flows can be mixed in later methods (stub periods, valuation dates within
  a year) without new discounting code.
− Fractional exponents, logarithms and exponentials use `rust_decimal` series functions:
  deterministic on every platform, but approximations. Accuracy is verified against a 50-digit
  reference and stated in the methodology pages; "exact" applies to rational operations only.
− Roots of the present-value function closer to each other than the solver grid spacing are
  not distinguished; the grid is documented.
