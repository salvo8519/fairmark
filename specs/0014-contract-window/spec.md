# SPEC-0014 — Contract window after 0.1.0-alpha.1: null and numbers, one code catalogue, statements contract

| | |
| --- | --- |
| Status | Approved — maintainer decisions of 2026-11-28; the spec becomes Approved when this pull request is merged |
| Milestone | M3 · Agent layer and v0.1 release |
| Owner | @salvo8519 |
| Formula basis | No formula: a contract change that alters no computed number. Basis: ADR 0014 G3, G4, G5, G6, G7, G8 and G10; the maintainer decisions of 2026-11-09 and 2026-11-28; RFC 6901 for pointers, RFC 8259 for JSON numbers, JSON Schema 2020-12 for the schemas. |
| Related | Issues #263, #266, #352 (in scope); #260, #353, #335 (related, out of scope); ADR 0009, ADR 0012, ADR 0013, ADR 0014, ADR 0018; SPEC-0005, SPEC-0007, SPEC-0012, SPEC-0013 |
| Commit scope | Existing scopes: `errors` (catalogue), `schemas`, `explain`, `statements`, `cli`, `py`, `tools`, `mcp`, `docs`, `i18n`, `specs`. No new scope. |

## Problem
Version 0.1.0-alpha.1 was published on 2026-11-28. Every input and output schema it carries is
now recorded in `schemas/index.json` with its sha256, and CI refuses an edit to a published
version. Three planned contract changes touch those schemas: null equals absent with one number
grammar (#266), one catalogue of error and warning codes (#352), and the statements contract
(#263). Made one at a time, they would move some methods two or three versions and make
integrators migrate three times. On 2026-11-28 the maintainer decided to collect them into one
contract window: one spec, one version bump per method that changes, one migration. This
applies after the alpha the decision of 2026-11-09 ("all contract changes, including breaking
ones, are collected before the v0.1 release: one version bump per method").

## Starting state (main, 2026-11-28)
- **Number grammar.** ADR 0014 G7 sets one grammar, `^-?\d+(\.\d+)?([eE][+-]?\d+)?$`. The core
  implements it (`is_decimal_text`, `crates/fairmark-core/src/reading.rs`), but the input
  schemas publish `SCHEMA_DECIMAL_PATTERN = ^-?\d+(\.\d+)?([eE]\d+)?$`, without the sign of the
  exponent. The engine accepts `"1e-1"` and `"1e+0"`, which the published pattern refuses.
  `"+0.1"`, `".1"`, `" 0.1"` and `"1.000,5"` are refused by both, with `FM-E001` `type`. The
  walk of the input recognises a decimal field by that pattern string.
- **Null.** G3 states that absent and `null` mean "not given" everywhere. The schemas apply it
  only in part. For example, the optional fields of `dates.schedule` admit `null`, but some
  defaulted fields of other methods do not (ADR 0014, Context). `statements.reorganise` uses
  `null` with its own meaning inside `adjustments[*].amounts`: "the adjustment does not apply
  to this year".
- **Floats.** A Python `float` given to the SDK (`fairmark.run`, `fairmark.run_json`, the
  method helpers) is sent as its shortest decimal text and raises no warning. `FM-W017` comes
  only from adapters where an outside library turns JSON numbers into floats: function calling
  with a dict, the MCP server and LangChain, which pass the pointers of those values to the core
  as the run option `binary_floats` (`docs/en/reference/warnings.md`, "Floats and the limit of
  FM-W017"). So the same float is warned on one channel and silent on another. R33 closes this
  gap.
- **Codes.** 27 error codes (`FM-E001` to `FM-E027`) and 17 warning codes (`FM-W001` to
  `FM-W017`). Error codes are variants of `FairmarkError` (`error.rs`) and warning codes are
  declared in `diagnostics.rs`. `docs/en/reference/errors.md` and `warnings.md` are maintained
  by hand and kept in step by tests. No channel exposes the catalogue at run time.
- **Where codes appear in schemas.** A method output schema describes `value` only. The warning
  `code` (pattern `^FM-W[0-9]{3}$`) and the error `code` (free string) live in
  `schemas/envelope/v1.json`. Inside `value`, codes appear only in the `{code, message}` cells
  of `analysis.sensitivity` v2 and `analysis.scenarios` v2.
- **Statements.** `statements.reorganise` is at v2 (SPEC-0005, SPEC-0012). Its contract is
  described in section C.

## Scope
**In scope:**
- #266: null equals absent in every input of every method; the G7 grammar published in every
  input schema; one grammar for the schema, the core and every adapter; no floats in money
  paths: a Python `float` given to any Python channel raises `FM-W017`, as on the other
  channels.
- #352, with the scope extension in the maintainer's comment of 2026-11-17: one catalogue
  table in `fairmark-core`; the reference tables and the schema code constraints generated from
  it; the catalogue exposed at run time by the CLI, Python, MCP and function calling, with a
  cross-channel test.
- #263: the v3 contract of `statements.reorganise` (section C).
- One version bump per changed method, one envelope version, one migration page.

**Out of scope:**
- #260, the output versioning rule and the version pin. Related: rule R30 below states how
  this window versions; #260 decides the general rule.
- #353, the compatibility ADR for method versions after v1.0.
- #335, per-method schema manifests.
- Any change to a computed number (see "Numerical change").

## User stories
- As an AI agent using a strict tool mode that sends `null` for every optional field, I want
  `null` to mean "not given" in every method, so that my calls are not refused.
- As an integrator, I want the schema pattern and the engine to accept the same numbers, so that
  validating against the schema never refuses an input the engine accepts.
- As an AI agent, I want to read the code catalogue from the channel I use, so that I can
  interpret a code without fetching the documentation.
- As an analyst, I want to give the years of a filing in the order printed, current year first,
  and get them valued in chronological order, so that a copied filing is never misread.
- As an integrator, I want to migrate once for all three changes.

## Requirements (EARS)

### A. Null equals absent and one number grammar (#266)
- **R1** — WHEN a field that is not required is given as `null`, THE ENGINE SHALL treat it
  exactly as if the key were absent, in every method and at every depth.
- **R2** — WHEN a defaulted field is given as `null`, THE ENGINE SHALL apply its documented
  default and list it in `meta.defaults_applied` with the field's pointer, as for an absent key.
- **R3** — IF a required field is given as `null`, THEN THE ENGINE SHALL return `FM-E001` with
  `reason` `missing` at the field's pointer and the fix of the missing field, not `reason`
  `type`. This changes the error report only. It moves no method version by itself, and the
  release notes state it (decision 4).
- **R4** — IF an element of an array is `null`, THEN THE ENGINE SHALL return `FM-E001` with
  `reason` `type` at the element's pointer. An array position cannot be "not given" (G6, no
  positional alignment; decision 5).
- **R5** — THE ENGINE SHALL compute `meta.input_hash` on a canonical input from which members
  given as `null` are removed, so that an input with a `null` member and the same input without
  the key have the same hash.
- **R6** — THE ENGINE SHALL publish, in every new input schema, a type that admits `null` for
  every property that is not required, and the schema `default` keyword for every defaulted
  field, holding the real default (G3).
- **R7** — THE ENGINE SHALL publish the G7 grammar `^-?\d+(\.\d+)?([eE][+-]?\d+)?$` as the
  `pattern` of every decimal field of every new input schema. The pattern SHALL come from the
  single definition the core uses to read a decimal string, so that the schema and the reader
  cannot diverge. No adapter SHALL apply another grammar.
- **R8** — WHEN any string is given as a decimal, THE ENGINE SHALL accept it if and only if it
  matches the published pattern.
- **R33** — WHEN a Python `float` is given to the SDK through any Python channel
  (`fairmark.run`, `fairmark.run_json`, the method helpers), at any depth of the input and in an
  array element too, THE SDK SHALL pass its pointer to the core in the run option
  `binary_floats`, and THE ENGINE SHALL add warning `FM-W017` at that pointer, as it does for
  function calling, MCP and LangChain. THE value SHALL still be read from its shortest decimal
  text, so no computed number changes. A Python `int`, a `Decimal` and a `str` SHALL raise
  nothing. (Numbered R33 so that the ids R1 to R32 stay stable; it belongs to section A,
  "no floats in money paths" of #266. This settles the point that ADR 0014 left open under
  "Not decided here".)

### B. One catalogue of error and warning codes (#352)
- **R9** — THE ENGINE SHALL hold one catalogue table in `fairmark-core` with one entry per error
  and warning code. Each entry has `code`, `kind` (`error` or `warning`), `meaning` (one
  sentence, English), `reasons` (the `reason` values the error can carry; empty for a warning),
  `pointer_root` (warnings: `input`, `envelope` or `null`, as the catalogue page states today;
  `null` for errors), `fix` (suggested fix, guidance), `methods` (the method ids that can emit
  it, or every method) and `added_in` (the engine version that introduced it).
- **R10** — THE ENGINE SHALL derive the code of every `FairmarkError` variant and every warning
  declaration from the catalogue table, or check it against the table in a test. A code emitted
  without an entry, or an entry with no emission site, fails the build.
- **R11** — WHEN the test suite runs, every pair of method and code it observes SHALL be
  declared in the entry's `methods`. A declared pair that is never observed SHALL be listed with
  a reason in the table, or fail the test. A code that every method can emit as it reads its
  input (`FM-E001`, `FM-W001`, `FM-W017`) is declared for every method (decision 13).
- **R12** — THE CLI SHALL provide `fairmark codes`, which writes the catalogue as JSON on
  standard output. `--errors` and `--warnings` restrict it to one kind; a positional code (for
  example `fairmark codes FM-E004`) returns that single entry.
- **R13** — THE Python SDK SHALL provide `fairmark.codes(kind=None, code=None)`, which returns
  the same catalogue.
- **R14** — THE MCP server SHALL expose the catalogue as the tool `fairmark_codes` and as the
  resource `fairmark://codes`. THE function-calling tools SHALL include `fairmark_codes` for
  every provider. Both take the same filters as R12.
- **R15** — THE ENGINE SHALL produce the catalogue response
  `{engine_version, entries: [...]}` in the core, with entries ordered by code (errors first).
  Every channel SHALL return it unchanged: the same JSON after canonical serialisation.
  Adapters only transport it (constitution IV).
- **R16** — IF a single code is requested that the catalogue does not hold, THEN THE ENGINE
  SHALL return the new error `FM-E028`, "unknown catalogue code", with `reason` `enum` and the
  known codes in `expected`, on every channel (decision 12).
- **R17** — THE build SHALL generate the "Codes" table of `docs/en/reference/errors.md` and the
  "Catalogue" table of `docs/en/reference/warnings.md` from the catalogue table. The prose of
  both pages stays hand-written. The Italian tables are generated in the same way, with the
  `meaning` and `fix` columns taken from `i18n/codes.it.yaml`, keyed by code, which
  `scripts/i18n.py check` keeps complete (decision 14). `scripts/docs_sync.py` SHALL fail when a
  generated table is stale.
- **R18** — THE schemas SHALL constrain every error and warning `code` with the pattern
  `^FM-E[0-9]{3}$` or `^FM-W[0-9]{3}$` and a reference to the catalogue schema. This applies to
  the envelope (warning `code`, error report and entries) and to the `{code, message}` cells of
  `analysis.sensitivity` and `analysis.scenarios`. The build SHALL generate the enums of every
  code into `schemas/codes/v<N>.json`, which has its own version in `schemas/index.json`. A new
  code bumps only the catalogue schema (decision 3). Clients still accept codes they do not
  know (G8).

### C. Statements contract, `statements.reorganise` v3 (#263)
Contract of v2 today (`crates/fairmark-core/src/statements/input.rs`, `validate.rs`):

| Topic | v2 today |
| --- | --- |
| Year order | `years` oldest first. `date` optional; without dates, the order given is taken as oldest first and a convention says so. Two consecutive dated entries must be one financial year apart, oldest first, otherwise `FM-E001` `conflict`. |
| Adjustments | `adjustments[]` tagged by `kind`, each with `amounts`, one entry per year aligned by position with `years`. `null` means "does not apply to this year". `lease` has one `LeaseAmounts` object per year. `debt_like_provision.interest_cost` is a second positional array. The output `AppliedAdjustment.index` is the position in `adjustments`. |
| Totals | A node with a total and parts (`NetFixedAsset`, `WithMaturity`, `Receivables`, `Equity`, `Provisions`, `Payables`, `DepreciationAndWriteDowns`, `OperatingWorkingCapital`, `ReclassifiedNetFinancialPosition`) is always an object, also for its total alone (`{"total": ...}`). |
| Unit | "Currency units of the statements, one unit per call, never scaled". The unit is not stated in the input or the result. |
| Current and non-current | Computed from a statutory year (the "beyond one year" parts). A reclassified year can give no current or non-current amounts other than the two parts of the net financial position. |
| Write-down treatments | `write_downs_of_receivables` and `write_downs_of_fixed_assets` default to `operating` when a statutory year gives an income statement. The schema `default` of `treatments` holds `null` values that mean "not given", which G3 does not admit. |
| Italian formats | `"1.000,5"` is refused as `FM-E001` `type`, with the generic fix. `"1.000"` is read as one. |

- **R19** — IF `years` has more than one entry and an entry has no `date`, THEN THE ENGINE SHALL
  return `FM-E001` `reason` `missing` at `/years/<i>/date` for each such entry, with the fix
  "give the closing date of every year" (G10.4). A single year may omit `date`.
- **R20** — WHEN every year has a date, THE ENGINE SHALL sort the years by date, ascending,
  whatever order they are given in. IF two years have the same date, THEN THE ENGINE SHALL return
  `FM-E001` `conflict` naming both. THE ENGINE SHALL apply the rule "one financial year apart"
  to consecutive years in sorted order.
- **R21** — THE ENGINE SHALL report the order in `explain.conventions` (one line listing the
  dates in chronological order and the input positions they came from). THE ENGINE SHALL return
  `value.years` in chronological order, each year carrying `input_pointer` (`/years/<i>` as
  given). Pointers into the input keep the positions given; pointers into the result use the
  chronological positions (decision 7).
- **R22** — THE ENGINE SHALL read `adjustments` as flat entries, one per adjustment and year:
  `kind`, `date`, `label`, `reason`, the fields of the kind, and `amount` (for `lease`, its seven
  amounts as fields of the entry; for `debt_like_provision`, `interest_cost` as one optional
  amount). A year with no entry has no such adjustment. With a single year and no date, entries
  omit `date`. An entry whose `date` matches no year is `FM-E001` `enum`, with the dates of the
  years in `expected`. Two entries with the same `kind`, `label` and `date` are `FM-E001`
  `duplicate`. `label` and `reason` repeat in each year's entry (decision 6).
- **R23** — THE ENGINE SHALL identify an applied adjustment in the result by `input_pointer`
  (`/adjustments/<k>`), replacing `index`.
- **R24** — WHEN a node with a total and parts is given as a decimal instead of an object, THE
  ENGINE SHALL read it as `{"total": <decimal>}`. Every pointer that names that total in errors,
  diagnostics and `aggregate_pointer` SHALL be the pointer of the node as sent. The canonical
  input SHALL expand the shorthand, so that both spellings give the same `meta.input_hash`. The
  shorthand applies to the nine node types listed above (decision 8).
- **R25** — THE ENGINE SHALL accept an optional root field `amounts_unit`, one of `units`,
  `thousands` or `millions`, which states the unit of every amount of the input and the result.
  THE ENGINE SHALL never scale an amount. It SHALL echo the unit in `value.amounts_unit` and in
  one line of `explain.conventions`, and SHALL NOT place it in `meta` (G4: nothing
  method-specific outside `value`). The field has no default. When it is not given,
  `value.amounts_unit` is `null` and the convention says that no unit was stated. No number
  depends on the unit, so G10.4 does not apply (decision 9).
- **R26** — THE ENGINE SHALL accept an optional `current_non_current` block in a reclassified
  balance sheet, with the fields of the current and non-current block of the result (SPEC-0005).
  It SHALL compute that block from the amounts given and list missing parts in
  `diagnostics.not_computed`. The statements contract of #263 applies to `statements.reorganise`,
  the only `statements.*` method in the registry. SPEC-0011 adopts it when that method is
  written (decision 10).
- **R27** — THE ENGINE SHALL publish `"default": "operating"` for
  `treatments.write_downs_of_receivables` and `treatments.write_downs_of_fixed_assets`, and no
  `default` keyword holding `null` values. The default SHALL be applied and reported in
  `meta.defaults_applied` only when a statutory year gives an income statement, as in v2, and
  `FM-W015` SHALL keep its current condition.
- **R28** — IF a decimal string contains a comma, or more than one point, THEN THE ENGINE SHALL
  return `FM-E001` `type` at its pointer, with `expected` stating the G7 form and a fix that
  rewrites the value (for example "write 1000.5 for 1.000,5"). The reader is shared, so this
  applies in every method. The wording of the fix is guidance, not contract (G5).
- **R29** — WHEN an amount of `statements.reorganise` is given as a string whose fractional
  part has exactly three digits, all zero (`"1.000"`, `"250.000"`), THE ENGINE SHALL accept it as
  written and add the new warning `FM-W018`. The warning names the field, the value read and the
  value an Italian reading would give (for example read 1, Italian reading 1000), as G10.5
  requires: a value that is plausible but unusual is warned, not refused. This is a warning, not
  a refusal, because `"12.500"` is a valid decimal and may be a legitimate amount in thousands
  (decision 11).

### D. Versioning
- **R30** — Every method whose published input or output schema changes in this window SHALL
  move exactly one version, and that version SHALL carry every change of the window for the
  method. This holds also when the change only widens what the schema accepts, because a
  published snapshot is immutable (decision 2). The snapshot files of earlier versions SHALL NOT
  be edited.
- **R31** — The envelope SHALL move from version 1 to version 2 once, for the changes of R18, and
  `meta.envelope_version` SHALL be 2.
- **R32** — THE documentation SHALL include one migration page listing, per method, the old and
  new version and every change a client must make.

| Method | Now | Next | Input change | Output change |
| --- | --- | --- | --- | --- |
| `analysis.goal_seek` | 2 | 3 | yes (A) | no |
| `analysis.scenarios` | 2 | 3 | yes (A) | yes (R18) |
| `analysis.sensitivity` | 2 | 3 | yes (A) | yes (R18) |
| `analysis.tornado` | 2 | 3 | yes (A) | no |
| `coc.beta_peers`, `coc.beta_relever`, `coc.cost_of_debt`, `coc.cost_of_equity`, `coc.wacc` | 1 | 2 | yes (A) | no |
| `dates.schedule` | 1 | 1 | no | no |
| `dates.year_fraction` | 1 | 1 | no | no |
| `dcf.fcfe` | 1 | 2 | yes (A) | no |
| `dcf.fcff` | 3 | 4 | yes (A) | no |
| `dcf.free_cash_flows` | 2 | 3 | yes (A) | no |
| `loans.amortisation_schedule`, `loans.instalment`, `loans.number_of_instalments` | 1 | 2 | yes (A) | no |
| `rates.convert`, `rates.real_nominal` | 1 | 2 | yes (A) | no |
| `ratios.capital_structure`, `ratios.covenants`, `ratios.debt_service`, `ratios.liquidity`, `ratios.profitability`, `ratios.working_capital` | 3 | 4 | yes (A) | no |
| `series.project` | 1 | 2 | yes (A) | no |
| `statements.reorganise` | 2 | 3 | yes (A, C) | yes (R21, R23, R25, R26) |
| `tv.compare` | 2 | 3 | yes (A) | no |
| `tv.exit_multiple`, `tv.gordon`, `tv.h_model`, `tv.limited_horizon`, `tv.liquidation`, `tv.normalised_income`, `tv.value_driver` | 1 | 2 | yes (A) | no |
| `tvm.annuity`, `tvm.future_value`, `tvm.mirr`, `tvm.perpetuity`, `tvm.present_value`, `tvm.xnpv` | 1 | 2 | yes (A) | no |
| `tvm.irr` | 3 | 4 | yes (A) | no |
| `tvm.npv`, `tvm.xirr` | 2 | 3 | yes (A) | no |
| Envelope | 1 | 2 | — | yes (R18) |

Totals over the 44 methods: 42 change their input schema, 3 their output schema, 3 both,
2 neither (`dates.schedule` and `dates.year_fraction` have no decimal field, and the optional
fields of `dates.schedule` already admit `null`). "Input change: yes (A)" rests on the decimal
pattern (R7), which every input with a decimal field carries. Task T1 of the plan confirms each
row with a script over `schemas/`, together with the fields where `null` is refused today
(R6). Output decimals use `^-?\d+(\.\d+)?$`, a subset of G7, and do not change. Most bumps
widen what the schema accepts, which G8 would call additive. They still take a new version
because the snapshot of the current version is immutable (decision 2). #260 may later define
additive releases. R3 and R33 change the error report and the warnings, not a schema, so they
add no row. The catalogue schema `schemas/codes/v1.json` is new (R18).

## Numerical change
This window must not change any computed number, and no expected value of a golden case changes.
These points could look like numerical changes. Each one is left to the maintainer:

1. **Year order.** An undated multi-year input that v2 read as oldest first is refused in v3
   (R19). A dated input given newest first, refused in v2, is valued in v3. No input valid in
   both versions changes a number. Golden cases with undated multi-year inputs need dates added
   to their inputs (decision 15).
2. **`meta.input_hash`.** The canonical input changes for `statements.reorganise` (flat
   adjustments, shorthand expanded, `amounts_unit`). Removing `null` members (R5) can change the
   hash of any input that sends `null`. The hash is not a financial number. Any golden case that
   asserts it is listed for approval (decision 15).
3. **Fields where `null` and absence differ today.** If T1 finds a field where `null` today has a
   meaning other than "not given" (as in the statements `amounts`), and a result would change, it
   is reported to the maintainer as a numerical change before any code is written.
4. **`amounts_unit`.** It never scales (R25), so no number depends on it.
5. **Python floats (R33).** The value is still read from its shortest decimal text, as today.
   Only `diagnostics.warnings` and the echo in `explain.conventions` gain `FM-W017`. The
   pointers travel as a run option, not as an input field, so `meta.input_hash` does not change.

## Conventions and parameters
| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Meaning of `null` in an input | not given (same as absent) | the only reading | ADR 0014 G3 |
| Decimal grammar of an input string | G7 pattern | the only grammar | ADR 0014 G7; RFC 8259 section 6 |
| Python `float` in an input | read from its shortest decimal text, with `FM-W017` | the only reading, on every channel | ADR 0014 G7; maintainer decision of 2026-11-28 |
| Order of statement years | chronological by `date` | the only order | ADR 0014 decision 3 of 2026-11-09 |
| Unit of the statement amounts (`amounts_unit`) | `units`, `thousands`, `millions` | none (not given, echoed as `null`; decision 9) | art. 2423 c.c. |
| Write-downs of receivables (B10 d) | `operating`, `investing` | `operating` | SPEC-0005 E3; IFRS 18 practice |
| Write-downs of fixed assets (B10 c) | `operating`, `investing` | `operating` | SPEC-0005 P1; OIC 12 paragraph 75 |

## Computability, defaults and checks
| Output | Inputs it needs | When an input is missing | Checks it takes part in |
| --- | --- | --- | --- |
| `/value/years` order (statements) | `/years/*/date` when more than one year | error `FM-E001` `missing` at each undated year (floor, G10.4) | one financial year apart, in sorted order |
| `/value/amounts_unit` | `/amounts_unit` | `null`, and the convention says that no unit was stated; not listed in `not_computed` (an echo, not an output) | none |
| Current and non-current block of a reclassified year | `/years/<i>/reclassified/balance_sheet/current_non_current/*` | not computed, `missing_input`, listing the fields | the block's own reconciliations, as for a statutory year |
| An adjustment for a year | an entry with that year's `date` | none applied for that year: nothing was given, nothing reported | `FM-E026` against what remains of the source, unchanged |
| B10 d and B10 c destinations | `/treatments/write_downs_of_*` | documented default `operating`, in `meta.defaults_applied`; `FM-W015` when the amount is not zero | a documented default does not exempt the reconciliations of the income statement |
| Catalogue response | none (and an optional filter) | the full catalogue | cross-channel identity (R15) |

## Acceptance criteria
- **AC1** (R1, R2) — Property test over the golden inputs of every method: each field that is
  not required, set to `null` in turn, gives the same envelope as the same input without the
  key, `meta.defaults_applied` included.
- **AC2** (R3) — For every required field of every method, an input with that field `null`
  returns `FM-E001` `missing` at its pointer.
- **AC3** (R4) — A `null` array element returns `FM-E001` `type` at the element's pointer, in
  every method with an array input.
- **AC4** (R5) — Property: `input_hash` is equal for an input and the same input with any
  optional member set to `null`.
- **AC5** (R6) — Conformance test over every new input schema: every property that is not
  required admits `null`, and every documented default appears as a `default` keyword with a
  value that is not `null`.
- **AC6** (R7, R8) — Conformance corpus (`"1"`, `"-0.5"`, `"1e-1"`, `"1E+3"`, `"+0.1"`, `".1"`,
  `"1."`, `" 0.1"`, `"1_000"`, `"1,000"`, `"1.000,5"`, `"1e"`, `"--1"` and more): the published
  pattern (checked with a JSON Schema validator), the core reader and the Python, CLI, MCP and
  function-calling channels give the same verdict for each string.
- **AC7** (R9, R10) — A test fails when a code is emitted without an entry, when an entry has no
  emission site, or when a `FairmarkError` variant or a warning declaration has a code that
  differs from the table.
- **AC8** (R11) — After the full test suite, the observed pairs of method and code are a subset
  of the declared pairs; each declared pair that is never observed carries a reason.
- **AC9** (R12 to R15) — Cross-channel test: `fairmark codes`, `fairmark.codes()`, the MCP tool,
  the MCP resource and the function-calling tool, each without a filter, with `--errors`, with
  `--warnings` and with one code, return identical JSON after canonical serialisation.
- **AC10** (R16) — Requesting `FM-E999` returns `FM-E028` with `reason` `enum` and the known
  codes in `expected`, on every channel.
- **AC11** (R17) — Editing an entry of the table without regenerating makes
  `scripts/docs_sync.py` fail. The generated tables list every code once.
- **AC12** (R18, R31) — The envelope v2 schema and the outputs of `analysis.sensitivity` v3 and
  `analysis.scenarios` v3 constrain `code` with the code patterns and the reference to
  `schemas/codes/v1.json`. That schema lists every code of the catalogue as an enum, and nothing
  else. `meta.envelope_version` is 2 in every result.
- **AC13** (R19) — A two-year input with one date missing returns `FM-E001` `missing` at that
  year's `date`. A one-year input without a date is accepted.
- **AC14** (R20, R21) — Property: any permutation of the years of a dated golden input gives the
  same `value`, except `input_pointer` and the order line of the conventions. Same dates are
  `conflict`.
- **AC15** (R22, R23) — Each statements golden input restated with flat adjustments gives the
  expected values of its v2 case. An unknown date is `enum` with the year dates; a repeated
  entry is `duplicate`.
- **AC16** (R24) — For every node with a total, the shorthand and `{"total": ...}` give the same
  envelope and hash; an error about the total names the node pointer.
- **AC17** (R25) — Each value of `amounts_unit` gives the same numbers, with the unit echoed in
  `value` and in one convention line.
- **AC18** (R26) — A reclassified year with the full block computes it; with a part missing, the
  outputs that need it are in `not_computed` with `missing_input`.
- **AC19** (R27) — The v3 schema carries `"default": "operating"` for both treatments; the
  default is reported only when a statutory income statement is given.
- **AC20** (R28, R29) — `"1.000,5"` and `"1.000.000"` return `FM-E001` `type` with the rewritten
  value in the fix. `"1.000"` in a statements amount is read as 1 and raises `FM-W018` at its
  pointer, naming the value read (1) and the Italian reading (1000). `"12.500"` and `"1.5"` raise
  nothing.
- **AC21** (R30, R32) — `schemas/index.json` shows exactly one new version for each method
  marked "yes" in the table, and none for the others. The migration page lists every one.
- **AC22** (R33) — A `float` in a nested field (for example `/terminal/tv.gordon/growth`) given
  to `fairmark.run`, to `fairmark.run_json` and to the method helper raises one `FM-W017` at that
  pointer.
- **AC23** (R33) — A `float` in an array element (for example `/cash_flows/2`) raises `FM-W017`
  at the element's pointer.
- **AC24** (R33) — An `int`, a `Decimal` and a `str` in the same positions raise no warning. The
  `value` of each result equals the `value` for the same input given as the float's shortest
  decimal text, and `meta.input_hash` is the same.
- **AC25** (R33) — Cross-channel test: the same input with the same floats gives identical
  `diagnostics.warnings` and `explain.conventions` through the Python SDK, function calling, the
  MCP server and LangChain.

## Interfaces
- Contract: follows ADR 0014 G1 to G10. No departure.
- Minimum input (G10): unchanged for every method. For `statements.reorganise`, one year with at
  least one amount, and a date for every year when there is more than one.
- Input schema changes: breaking for `statements.reorganise` (R19, R22). Widening for the other
  41 inputs (R6, R7). Each takes a new version (R30, decision 2).
- Output changes: `statements.reorganise` (`input_pointer` on years and adjustments,
  `amounts_unit`, the block of R26); the two analysis outputs and the envelope (R18).
- New adapter entry points: CLI `codes`, `fairmark.codes`, MCP `fairmark_codes` tool and
  resource, function-calling `fairmark_codes`.
- New codes: warning `FM-W018` (R29) and error `FM-E028` (R16). `FM-W017` is now also emitted
  for Python floats (R33): its condition widens, and its code and meaning stay the same.
- New schema: `schemas/codes/v1.json`, the catalogue enums, with its own entry in
  `schemas/index.json` (R18).
- Python SDK: every Python channel collects the pointers of `float` values and passes them in
  `binary_floats` (R33), as `fairmark.tools.dispatch` does today. The adapter never writes to
  the envelope.
- Roadmap: the pull request that approves this spec sets `spec: "0014"` on tasks #263, #266 and
  #352 (every spec must be referenced by a roadmap task).

## Golden cases
No expected value changes. Statements cases need their inputs restated in the v3 shape (dates
added, flat adjustments). Cases of any method that record the method version or
`meta.input_hash` may need updating. These are existing protected cases. Task T1 produces their
list, and the maintainer approves it before any golden file is edited (decision 15). New cases
for R19 to R29 are added with an independent source: the v2 case of the same data.

## Documentation impact
- English: `docs/en/reference/errors.md` and `warnings.md` (generated tables, new prose for the
  catalogue channels); the methodology page of `statements.reorganise` (dates, order,
  adjustments, shorthand, unit, reclassified block); the CLI, Python, MCP and function-calling references
  (`codes`); a new migration page for the window; ADR 0014 Context, with a note that the
  pattern now matches.
- `docs/en/reference/warnings.md`: the section "Floats and the limit of FM-W017" is rewritten.
  A float is now warned on every channel, the Python SDK included, and the value is still read
  from its shortest decimal text. The `FM-W017` row of the catalogue changes its "Methods"
  column to "every method, on every channel that receives a float". Two new rows: `FM-W018`,
  and in `errors.md` `FM-E028`. These rows are generated from the catalogue (R17).
- `docs/en/reference/errors.md`: the "Number grammar" paragraph drops the sentence that the
  schema pattern "gains the sign with issue #266".
- Generated: `schemas/` (new versions, envelope v2, `schemas/codes/v1.json`),
  `docs/en/reference/methods.md`, the Python API reference.
- Italian: every page above through `/translate`, the FM-W017 section and row included, and the
  generated tables from `i18n/codes.it.yaml` (decision 14).
- Release notes, fragments in `changes/` for analysts:
  - "null is read as not given in every method; one list of codes, available from every
    channel; statements accept the years in any order with their dates, adjustments per year,
    totals written as one number, and an amounts unit";
  - "a Python float given to Fairmark now raises warning FM-W017 on every channel, as it
    already did through function calling and MCP; the numbers do not change; pass `Decimal` or
    strings for computed values".
- CHANGELOG entries with `!` for the breaking statements change.

## Sources
- ADR 0014 G3, G4, G5, G6, G7, G8, G10 and the maintainer decisions of 2026-11-09: null,
  envelope, errors, shapes, grammar, versioning, floor.
- ADR 0009 (method versions, immutable snapshots), ADR 0012 (status), ADR 0013 (error reading),
  ADR 0018 (sources policy).
- RFC 6901, JSON Pointer: every pointer.
- RFC 8259, section 6: JSON number grammar, which the G7 string grammar extends.
- JSON Schema 2020-12, Validation: `enum` (6.1.2), `pattern` (6.3.3), `default` (9.2).
- ISO 8601-1:2019, calendar date `YYYY-MM-DD`: `date` of years and adjustments.
- Codice civile art. 2423, sixth paragraph: statements in euro units without decimals, notes
  allowed in thousands of euro. This supports `amounts_unit`.
- Codice civile art. 2424: amounts "beyond the next financial year", the basis of the current
  and non-current block (SPEC-0005).
- Unicode CLDR, locale `it`: comma as decimal separator and point as grouping separator
  (to verify: version and data file). This supports R28 and R29.
- IFRS 18 and IAS 1, current and non-current classification (to verify: paragraphs after
  IFRS 18 replaces IAS 1). This supports the naming of R26.
- SPEC-0005 decisions E3 and P1, and OIC 12 paragraph 75: the write-down defaults.

## Decisions of 2026-11-28
Taken by the maintainer on the open questions of the draft. Each decision is now part of the
requirements and tables above.

1. **Issue texts:** the bodies of #263 and #352, the maintainer comment of 2026-11-17 on #352
   included, are fully covered. #266 adds "no floats in money paths", now R33.
2. **Bump on widening:** every changed snapshot takes a new version, also when the change only
   widens what it accepts (R30).
3. **Code constraint:** code patterns inline, with enums in a separate
   `schemas/codes/v<N>.json` that has its own version (R18).
4. **`null` on a required field:** `reason` `missing`, with no method version for this change
   alone, stated in the release notes (R3).
5. **`null` array elements:** refused in every method (R4).
6. **Flat adjustments:** one entry per adjustment and year, keyed by `date` (R22).
7. **Output years:** chronological, each with `input_pointer` (R21).
8. **Node shorthand:** a decimal at the node is its total, for the nine node types (R24).
9. **`amounts_unit`:** optional, no default, never scales, echoed as `null` when not given
   (R25).
10. **Reclassified current and non-current block:** fields mirror the result block, and the
    contract applies to `statements.reorganise` only (R26).
11. **`"1.000"` in statements amounts:** warning `FM-W018`, not a refusal (R29). This
    deliberately replaces the word "refused" in the texts of #263 and #266: `"12.500"` is a
    valid decimal and may be a legitimate amount in thousands. The warning names the field, the
    value read and the value an Italian reading would give. Strings with a comma or with more
    than one point stay refused (R28).
12. **Unknown code and names:** new error `FM-E028`; names `fairmark codes`,
    `fairmark.codes()`, `fairmark_codes` and `fairmark://codes` confirmed (R12 to R16).
13. **`methods` of an entry:** declared in the table, checked so that the observed pairs are a
    subset of the declared ones, with "every method" for reading codes (R11).
14. **Italian generated tables:** `meaning` and `fix` taken from `i18n/codes.it.yaml` (R17).
15. **Golden cases:** the maintainer approves the list that task T1 produces before any golden
    file is edited, with expected values unchanged ("Golden cases").

Also decided on 2026-11-28 (#266): a Python `float` given to any Python channel raises
`FM-W017` at its pointer, as on the other channels, and is still read from its shortest decimal
text (R33).

The two sources marked "to verify" (Unicode CLDR locale `it`; IFRS 18 and IAS 1 paragraphs)
remain flagged. They are confirmed before any public page cites them (ADR 0018).
