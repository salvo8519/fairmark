# SPEC-0010 — Tasks

Rules: test first; one task per commit; `[P]` tasks touch disjoint files and can run in
parallel; tick a task only when `just check` passes. Commits cite the requirements
(`feat(dates): accrual period schedule [SPEC-0010 R1, R2]`); the pull request body says
`Closes #206` (roadmap task #206, spec "0010", already in `roadmap/roadmap.yaml`). No existing
golden file is edited; if a task would change one, stop and ask the maintainer.

## Preparation
- [x] T1 Move the spec Status to `Implementing`; add `series` to the commit scopes of
      `scripts/conventions.py` (and its test).
- [ ] T2 Oracle dependencies: QuantLib-Python and python-dateutil pinned to exact versions in the
      `oracles` dependency group (`pyproject.toml`, `uv.lock`), per
      `docs/en/project/dependencies.md`; `security-reviewer` review of the change.
- [x] T3 [P] Reference generators, independent of the engine: `scripts/reference_schedule.py`
      (each schedule built with python-dateutil and with QuantLib `Schedule`, `Period(p,
      Months)`, `NullCalendar`, `Unadjusted`, forward, `endOfMonth`; fails on any disagreement;
      prints both library versions) and `scripts/reference_series.py` (50-digit Python
      `decimal`: recurrence, linear rule, the three bases for p dividing 12 and p multiple of
      12, `tvm.xnpv` of the end-to-end case); extend `scripts/reference_time.py` and
      `scripts/reference_loans.py` to m = 6 and m = 3.

## Tests (write first, they must fail)
- [ ] T4 [P] Golden cases `tests/golden/dates.schedule/` (new folder): the fourteen schedules
      of AC1, one case per day count (AC3), the error cases of AC4 (`expect_error`); `source`
      names QuantLib and python-dateutil with their pinned versions (R1 to R7).
- [ ] T5 [P] Golden cases `tests/golden/series.project/` (new folder): AC6, AC7 (Damodaran
      Illustration 15.3 and the 50-digit reference), AC8, AC9 (named frequencies,
      `period_months` 1 to 6, 12, 24, and the refusals for 5 and 18), AC10 including the
      end-to-end `tvm.xnpv` case, AC12; `source` names the reference script (R9 to R18).
- [x] T6 [P] New golden files only in `tests/golden/rates.convert/`, `tests/golden/tvm.xnpv/`
      and `tests/golden/loans.instalment/`: 0.0735 `annual` to `bimonthly` and to
      `four_monthly`, `tvm.xnpv` with `four_monthly`, `loans.instalment` with `bimonthly`
      against form (b), `FM-E003` at −3 with `four_monthly` (AC14). If the golden hook blocks a
      new file in an existing folder, stop and ask the maintainer.
- [ ] T7 Rust unit and property tests: frequency mapping agreement (R20); AC2 properties with
      named frequencies and `period_months` 1 to 36; AC5 descriptions and bounded `explain`;
      AC11 properties including the `dcf.fcff` stage equality; AC13 `explain` items and error
      messages with pointer and fix; AC14 round-trips over the seven compounding values;
      `period_months` `u32::MAX` with 10000 periods returns `FM-E009` (AC4).

## Implementation
- [x] T8 Shared frequency: `crates/fairmark-core/src/frequency.rs` (`Frequency`,
      `PeriodLength`, exactly-one and at-most-one validation); `Compounding` and
      `PaymentFrequency` gain `bimonthly` and `four_monthly`, appended, periodic values derived
      from `Frequency`; labels in `explain.conventions` (R20, R21). `uv run just generate`
      regenerates the current schemas of the affected methods: confirm the diff only adds the
      two values and the schema check accepts it as additive, no new version in
      `schemas/index.json` (R22, AC15); otherwise stop and ask the maintainer. Updated
      methodology pages of the affected methods in the same commit (docs-sync impact). Every
      existing golden case passes unchanged.
- [ ] T9 [P] ADR 0008 amendment note on §4: the compounding list gains `bimonthly` and
      `four_monthly`, defined once with the schedule frequencies (SPEC-0010 R20, Decision 5).
- [ ] T10 `dates.schedule` types in `dates.rs`: `ScheduleInput`, `ScheduleOutput`, doc comment
      on every field with units and the [start, end) phrases, the name-to-tenor mapping in the
      descriptions of `frequency` and `period_months` and in the method `DESCRIPTION` (R1, R3,
      R5, R6).
- [ ] T11 `dates.schedule` calculation: `add_months` in `i64` with the end-of-month rule,
      `build_schedule`, year fractions through `DayCount::year_fraction`, errors with pointers
      and fixes, `explain` of R8 via `schedule_explain`; registry line; `FM-E001` and `FM-E009`
      cases in `docs/en/reference/errors.md` (R1 to R8). T4 and the schedule part of T7 pass.
- [ ] T12 `series.project` types in the new theme `crates/fairmark-core/src/series.rs`:
      `ProjectInput`, `BasePeriod`, `GrowthRule`, `GrowthBasis`, `Timeline`, `ValueTiming`,
      output rows; `dated_values` uses the dated-flow type of `tvm` (R14 to R16).
- [ ] T13 `series.project` calculation: timeline resolution through `build_schedule`, growth
      steps, linear expansion, basis resolution for p dividing 12 (through `equivalent_rate`),
      p multiple of 12 (integer power) and other p (`per_period` only), recurrence period by
      period, rows, `values`, `dated_values`, negative base warning, `explain` of R19; registry
      line; error catalogue cases (R9 to R19, R24). T5 and the rest of T7 pass.
- [ ] T14 `uv run just generate`: `schemas/dates.schedule/v1.json`,
      `schemas/series.project/v1.json`, `schemas/index.json`, `docs/en/reference/methods.md`.

## Adapters and docs
- [ ] T15 [P] Python helpers: `fm.dates.schedule` in `python/fairmark/dates.py` and the new
      `python/fairmark/series.py` with `fm.series.project` (exported from the package);
      `Decimal` or strings only, never `float`; docstrings with an example; tests of the
      helpers.
- [ ] T16 [P] Oracle tests (Python, `oracles` group, CI matrix): random schedules against
      python-dateutil and QuantLib, every named frequency and `period_months` 1 to 36, year
      fractions against the QuantLib day counters (AC2, AC3).
- [ ] T17 [P] Methodology pages `docs/en/methods/dates-schedule.md` and
      `docs/en/methods/series-project.md` (docs-writer): [start, end) first with a plain-text
      timeline, the table of names and tenors and `period_months`, the end-of-month examples,
      the growth-basis comparison and the rule for other period lengths, the chained example
      `dates.schedule` → `series.project` → `tvm.xnpv` with the 1 April and 31 March dating,
      "Verified against"; the sources read for the terms marked "to verify".
- [ ] T18 [P] Navigation and catalogues: `zensical.toml`, `docs/en/methods/index.md`,
      `i18n/method_titles/it.yaml`, `i18n/glossary.yaml`, `skills/fairmark/SKILL.md`
      (docs-writer).
- [ ] T19 [P] Release-note fragment for #206 in `changes/` (per `changes/README.md`): two new
      preview methods and two new frequencies, in plain language for analysts.
- [ ] T20 Italian edition with `/translate` (translator), using `i18n/glossary.yaml`;
      `uv run just i18n` clean.
- [ ] T21 Docs coverage and sync green: `scripts/docs_sync.py coverage` and `impact`,
      `uv run just docs`.

## Verification
- [ ] T22 Independent numerical audit (numerical-auditor): recompute the golden cases of both
      methods and of the shared frequencies from the sources, check every convention in
      `explain.conventions`, determinism, the accuracy statements and the unchanged results of
      every existing value (AC15); fixes in their own commits.
- [ ] T23 Code review (code-reviewer) of the full diff against AGENTS.md, the constitution and
      this spec: decimal-only arithmetic, no adapter logic, schema docs, explain completeness,
      errors with pointers and fixes, a test for every acceptance criterion; fixes in their own
      commits.
- [ ] T24 Method evidence: entry `"0010"` in `specs/method-evidence.yaml` with methods
      `dates.schedule` and `series.project`, `numerical_audit: T22`, `code_review: T23`; both
      methods stay `preview` (not in `AVAILABLE`); `python scripts/method_status.py check`
      green. Promotion is a separate maintainer decision.
- [ ] T25 Pull request: `uv run just check` green, `python scripts/roadmap.py check` green,
      body `Closes #206`, every requirement cited in the commits; reviewers named in the body.
