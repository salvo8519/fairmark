# SPEC-0013 design: golden cases assert values, schema snapshots assert shape

| | |
| --- | --- |
| Status | Approved with its ten recommendations (maintainer, 2026-11-17); the requirements are in [`spec.md`](spec.md) |
| Issue | #351, decided by the maintainer on 2026-11-17 |
| Related | #259 and #361 (one envelope, 131 golden files migrated), #266 (null equals absent, one number grammar), #263 (statements contract), #352 (one catalogue of codes), ADR 0014 G4, G5, G8, SPEC-0012 |
| Base | `main` at `e9669cb`; 666 golden files, 44 methods |
| Tooling | `scripts/golden_values_migration.py` (`inventory`, `list`, `apply`, `verify`), added with this design |
| Tier | Substantial when implemented (test runners and the golden contract); the migration itself changes no expected value and needs the maintainer's named approval of the file list |

## Context and goal

SPEC-0012 (#361) changed no number but touched 131 golden files. The cause was not that golden
cases asserted shape for its own sake: almost every assertion is a value. They addressed those
values through the layout of the envelope: the position of an entry in
`meta.defaults_applied`, `diagnostics.not_computed` or `diagnostics.checks`, the part of the
envelope a list lives in, the name of the field that holds a pointer. When the layout moved,
the address of every value moved with it.

Goal: the next contract changes (#266, #263) touch the schema snapshots and few golden cases.
Golden cases address values by what they are (the output, the input field that received a
default, the relation id, the error code and pointer); the schema snapshots and one
conformance test own the layout.

Constitution check: I (determinism) is unaffected: no computation changes. VI is strengthened:
expected values stay independent and verbatim; no expected value is taken from the engine. IX
drives two choices below: one way to address each node, and an explicit meaning for `null`.

## 1. Inventory

Produced by `python scripts/golden_values_migration.py inventory`, which reads every golden case,
runs the engine on its input to know which lists the method fills, and classifies each
assertion against the current output snapshot (`schemas/<method>/v<N>.output.json`) or the
envelope snapshot (`schemas/envelope/v1.json`). It changes nothing.

### 1.1 Assertion keys in use

Golden files: 666 (394 value cases, 272 error cases). Assertions: 5,517.

| Section | Reads | Files | Assertions | VALUE | SHAPE | WORDING |
| --- | --- | --- | --- | --- | --- | --- |
| `expect_error` | error code | 272 | 272 | 272 | 0 | 0 |
| `expect` | `value` | 394 | 3,779 | 3,655 | 118 | 6 |
| `expect_length` | error report (all 8 are `/errors`) | 8 | 8 | 8 | 0 | 0 |
| `expect_meta` | `meta` (all are `/defaults_applied/N/...`) | 24 | 607 | 605 | 0 | 2 |
| `expect_diagnostics` | `diagnostics` | 99 | 766 | 729 | 0 | 37 |
| `expect_diagnostics_length` | `diagnostics` | 16 | 26 | 22 | 4 | 0 |
| `expect_error_report` | error report | 11 | 59 | 57 | 0 | 2 |

Other top-level keys: `method`, `description`, `source`, `input` (666 each), `tolerance` (394).

| Class | Kind | Assertions | Files |
| --- | --- | --- | --- |
| VALUE | number | 2,921 | 364 |
| VALUE | pointer the user acts on | 860 | 130 |
| VALUE | identifier or code not enumerated in the schema (check ids, destinations of defaults, technique ids, `expected` lists) | 467 | 103 |
| VALUE | enumerated code or status | 365 | 150 |
| VALUE | null (not computed or undefined) | 278 | 67 |
| VALUE | error code (`expect_error`) | 272 | 272 |
| VALUE | date | 144 | 21 |
| VALUE | count (no entry beyond those asserted) | 30 | 24 |
| VALUE | input echo whose position is a computed rank (`ranking`) | 6 | 2 |
| VALUE | warning or error code inside a list | 5 | 3 |
| SHAPE | identity of an entry: engine label (`label`, `name` at a position) | 96 | 19 |
| SHAPE | identity of an entry: input text echoed (`name`, `label`, `id`) | 22 | 15 |
| SHAPE | count of a list the method never fills | 4 | 2 |
| WORDING | `reason` text (not computed, defaults applied) | 39 | 19 |
| WORDING | `basis_note`, `not_tested_reason`, `fix` text | 8 | 6 |

How the assertions are addressed:

| Section | Plain path | Position in a list whose items carry a natural key | Ordinal position only | Both |
| --- | --- | --- | --- | --- |
| `expect` | 1,196 | 1,377 | 1,137 | 69 |
| `expect_meta` | 0 | 607 | 0 | 0 |
| `expect_diagnostics` | 0 | 578 | 0 | 188 |
| `expect_diagnostics_length` | 21 | 5 | 0 | 0 |
| `expect_error_report` | 21 | 25 | 8 | 5 |
| `expect_length` | 8 | 0 | 0 | 0 |

A natural key is a field of the list item that can name it (`id`, `pointer`,
`source_pointer`, `variable_pointer`, `aggregate_pointer`, `code`, `name`, `label`, `date`,
`method`). Lists most indexed by position: `value/years` 1,189 assertions, `value/periods` 631,
`meta/defaults_applied` 607, `diagnostics/not_computed` 446 (plus 163 into its
`missing_any_of`), `diagnostics/checks` 321, `value/covenants` 190, `value/rows` 160,
`value/lines` 154, `value/composition` 119, `error/errors` 30.

Finding: 94 % of assertions are values. What made #361 expensive is the 1,437 value
assertions addressed by envelope layout (`expect_meta`, `expect_diagnostics`,
`expect_diagnostics_length`, `expect_length`, `/errors/N` of `expect_error_report`), in 119
files. Assertions into `value` were touched in #361 only where a field of `value` was renamed.

### 1.2 Classification rule

Applied in this order to each assertion `(section, key, expected)`; the first rule that matches
decides. "Schema node" is the node of the current output or envelope snapshot at `key`, with
`$ref`, `anyOf`, `oneOf` and `allOf` followed.

1. `expect_error`: VALUE, the error code.
2. Path kind (recorded, does not decide the class): plain, position in a list with a natural
   key, ordinal position, or both.
3. A length section (`expect_length`, `expect_diagnostics_length`): SHAPE when the expected
   count is `0` and no golden result of the method ever fills that list (the list is empty by
   construction, so its length is not a result); VALUE otherwise (the list has no entry beyond
   those asserted: completeness of errors, warnings, outputs not computed, alternatives).
4. Expected `null`: VALUE, an output not computed or undefined.
5. Leaf field in `fix`, `message`, `reason` (except `reason` of an error entry, which is an
   enumerated code), `basis_note`, `not_tested_reason`: WORDING. The schemas say so for the
   first three ("its wording may change between releases", "guidance, not contract").
6. A decimal string where the schema node is a decimal, integer or unknown: VALUE, number.
7. `FM-E...` or `FM-W...`: VALUE, code.
8. Schema node with `enum` or `const` (schemars `oneOf` of `const`), or `reason` of an error
   entry: VALUE, enumerated code or status.
9. `YYYY-MM-DD`: VALUE, date.
10. Inside a list whose order the method computes (`ranking`): VALUE, the position is the
    result.
11. Starts with `/`: VALUE, a pointer the user acts on.
12. Leaf `label`, `name` or `id` inside `value`: SHAPE. At a fixed position it says which entry
    sits there; the value of the entry is asserted by its sibling fields.
13. Anything else: VALUE, an identifier or code the schema does not enumerate.

### 1.3 Borderline cases

- **`null` (278 assertions, 67 files).** The fact "not computed" is a value; whether it is
  written `null` or the key is absent is shape. Today the runners fail a missing key even when
  `null` is expected, so a change of encoding (#266, second reading) touches all 67 files.
  Section 2 separates the two.
- **Identity labels in `value` (118).** `/years/0/.../composition/2/label: Cash (C.IV)`
  identifies the line whose `amount` is asserted next to it. Engine labels are also wording
  ("Financial debt (D)"). They stay in the migration (they are in `value`); new cases should
  prefer the `source_pointer` of the line, which is a value.
- **WORDING (47).** Some carry a number (`reason: equity is negative (-200)`); `fix` names the
  closest accepted field, a behaviour decided on 2026-11-17. The migration keeps all of them
  (it drops nothing); whether new cases may assert them is open question 2.
- **Counts.** 30 are VALUE (no error, warning or output not computed beyond those asserted). 4
  are SHAPE (`/checks: 0`, `/warnings: 0` or `/not_computed: 0` in two `ratios.*` and
  `analysis.*` cases, for lists no golden result of the method fills). The SHAPE verdict is
  empirical, not declared by the method.
- **Identifiers not enumerated (467).** Check ids (`balance_sheet_identity`), destinations of
  defaults (`operating_fixed_assets`), terminal techniques: values, but the schema types them
  as free strings. Enumerating them belongs to #352.
- **Top-level fields of the error report** (`/pointer`, `/reason`, `/expected/N`,
  `/pointers/N`, `/missing_any_of/N/N`, 29 assertions): they repeat the first entry of
  `errors`, which is the error the typed API raises (`e.code`, `e.pointer`). VALUE, and a plain
  path: they stay where they are.
- **Order of envelope lists.** Asserting `/not_computed/2/pointer` asserts membership and the
  documented order at once. After the change, membership stays in golden cases and order moves
  to an invariant test (section 3.4).

## 2. The rule for golden cases

### 2.1 What a golden case may and may not assert

May assert, each from an independent source:

- numbers, dates, codes (errors, warnings, reason codes, statuses, enumerated values, check
  ids), pointers the user acts on;
- that an output is not computed (`null` in `value`, or an entry of `expect_not_computed`);
- the completeness of a list whose entries are results: errors, warnings, outputs not
  computed, checks (`expect_counts`);
- the order of a list when the order is itself the result (years, periods, schedule rows,
  `ranking`), through positions in `value`.

May not assert:

- where a list lives in the envelope, the name of a field that holds an identity, or the
  position of an entry in a list whose order is a presentation rule (defaults applied,
  diagnostics, errors): these are the layout the schema snapshots own;
- presence of keys, envelope structure, `meta` bookkeeping (`envelope_version`,
  `engine_version`, `input_hash`);
- wording (`message`, `fix`, the `reason` of an output not computed or of a default), unless
  the maintainer decides otherwise (open question 2).

### 2.2 How values are addressed

**Inside `value`: RFC 6901 pointers relative to `value`, as today (`expect`).** The layout of
`value` is the method's output contract: renaming or moving a field there is a breaking output
change with a method version bump (ADR 0014 G8), and the golden cases of that method are
touched deliberately. Positions in `value` are mostly meaningful (period `t`, year order,
schedule row, rank).

**Envelope lists: semantic sections keyed by the identity of the entry.** The runner, not the
golden file, knows where each list lives in the current envelope version. Each section maps the
key of an entry to a mapping of RFC 6901 pointers relative to that entry, compared like
`expect`; `{}` asserts only that the entry exists.

| Section | List (envelope v1) | Key of an entry | Example |
| --- | --- | --- | --- |
| `expect_defaults` | `meta.defaults_applied` | `pointer` (input pointer) | `/years/0/statutory/balance_sheet/cash: {/value: net_financial_position_cash}` |
| `expect_not_computed` | `diagnostics.not_computed` | `pointer` (result pointer) | `/value/interest_cover_ebitda: {/missing_any_of/0/0: /ebitda_definition}` |
| `expect_warnings` | `diagnostics.warnings` | `code` and `pointer`, one space between, `-` for a null pointer | `FM-W012 /base_input/net_financial_position: {}` |
| `expect_checks` | `diagnostics.checks` | `pointer` and `id`, one space between | `/value/years/0 funds_invested_equals_sources: {/status: reconciled}` |
| `expect_errors` | error report `errors` | `code` and `pointer`, one space between, `-` for a null pointer | `FM-E001 /rat: {/reason: unknown_field, /expected/0: rate}` |
| `expect_counts` | a semantic list name (`errors`, `not_computed`, `warnings`, `checks`), or a pointer into `value` | n/a | `errors: '4'` |

A key that names no entry, or more than one, fails the case. Codes and ids never contain a
space and a pointer never starts with a space, so a key reads one way only. A count inside an
entry is written `/missing_any_of#count`.

`expect_error` keeps the code of the error raised; `expect_error_report` keeps only the
top-level fields of the report (the first error, as the typed API raises it); `/errors/N/...`
keys move to `expect_errors`. After the migration the runners refuse `expect_meta`,
`expect_diagnostics`, `expect_diagnostics_length`, `expect_length` and `/errors/...` keys in
`expect_error_report`.

**`null` means "not computed": the value is `null` or the key is absent.** The parent of the
key must exist, and the key must be declared by the method's output schema; that check replaces
the presence check as the guard against a typo in a key. The meaning of `null` in the result is
decided by #266; the golden cases no longer depend on its encoding.

### 2.3 Semantic paths inside `value`: evaluated, not recommended now

A selector syntax such as `/years/[label=2024]/balance_sheet/...` would survive a reorder or
an insertion in a keyed list of `value`. Costs:

- one resolver in each runner (Rust and Python), escaping (`/` inside a key value is `~1`, so
  `/defaults_applied/[pointer=~1years~10~1statutory...]`), uniqueness rules;
- 1,446 assertions in keyed positions rewritten (1,189 of them `/years/N`), for no current
  defect;
- two ways to address the same node, which principle IX asks to avoid.

Benefit: only when a contract change reorders or inserts entries of a keyed list of `value`.
Neither #266 nor #263 as described does so for existing cases (their years are already oldest
first; the engine sorting them changes only `error-dates-newest-first.yaml`). Recommendation:
do not add selectors; revisit if a contract change reorders a keyed list of `value`.

### 2.4 Is it worth it

Yes for the envelope sections and the `null` rule; no for selectors in `value`.

| | Cost | Benefit |
| --- | --- | --- |
| Envelope sections | about 150 lines in each runner, one migration of 119 files, the skill and `tests/README.md` | #361 in retrospect: 53 files instead of 131; #263: 13 fewer files (section 5) |
| `null` rule | about 30 lines in each runner, plus the schema check of keys | #266, second reading: 0 files instead of 67 |
| Conformance test | one Python test module (section 3) | catches every layout defect the golden cases stop catching |
| Selectors in `value` | section 2.3 | none foreseen |

## 3. The schema-conformance test

### 3.1 What exists

`python/tests/test_output_schemas.py::test_golden_results_match_their_result_schema` validates
the 394 successful golden results against `fm.result_schema(method)`. It has three gaps: the
schema comes from the engine, not from the committed snapshots; error cases are not validated;
and the schemas are open (133 of the 149 object schemas of the current output and envelope
snapshots have no `additionalProperties`), so a field added, or a field renamed with the old
name left in place, passes.

### 3.2 Proposed test

`python/tests/test_golden_conformance.py`, replacing the test above, for every golden case:

- success: the envelope against `schemas/envelope/v<meta.envelope_version>.json`; `value`
  against `schemas/<method>/v<meta.method_version>.output.json`, both read from disk, so the
  result is checked against the version it claims;
- error: `{"error": <report>}` against the definition named by `x-fairmark-error` in the
  envelope snapshot;
- closed mode: at test time, `unevaluatedProperties: false` is added to every object schema
  that declares `properties`, except a branch of `anyOf`, `oneOf` or `allOf` (the parent closes
  it, so a flattened enum such as `ScenarioResult` keeps its sibling fields). The snapshots
  themselves stay open: a new output field is additive (ADR 0014 G8);
- formats asserted with `FormatChecker` (`date`);
- for an analysis method, each inner value (`x-fairmark-output-of`) against the inner method's
  snapshot (open question 7);
- every assertion key of the case is declared by the schema (the guard of the `null` rule).

Prototype on `main` (not committed): 394 success results and 272 error reports validated,
open and closed: 0 failures. The migration lands on a green base.

### 3.3 Validator

`jsonschema` (Python), already used by `test_output_schemas.py`, `test_error_report.py` and
`test_tool_definitions.py`, and already in `uv.lock` at 4.26.0, which it reaches only through
the `mcp` extra. Dependency policy (Adapters and tooling):

- actively maintained: release 4.26.0 on 2026-01-07, within twelve months;
- licence MIT, on the allow-list of `deny.toml`;
- reason: JSON Schema 2020-12 with `$ref`, `unevaluatedProperties` and formats is not a few
  lines of our own code.

Action: declare `jsonschema>=4.23` in the `dev` dependency group, so the tests do not depend on
an optional extra. No new package enters the lock.

Rejected: the Rust `jsonschema` crate as a dev-dependency of `fairmark-core`. It would be a new
dependency for a check Python already performs, and its default features resolve references
over the network and the file system, which the policy forbids for dev-dependencies of the
core. The Rust golden runner keeps asserting values only.

### 3.4 What it catches that golden cases will no longer catch

| Defect | Caught today by | Caught after by |
| --- | --- | --- |
| An envelope key missing or renamed (`diagnostics`, `meta.defaults_applied`) | Golden cases, by accident of their paths | Conformance test (`required`) |
| A diagnostics list absent instead of empty | Golden cases with a length | Conformance test (`required`) |
| An entry of a warning, check, output not computed or default with a field missing, renamed or extra | Golden cases at its position | Conformance test (closed mode) |
| A field of `value` added or renamed without a new snapshot | Nobody (snapshots are open) | Conformance test (closed mode) plus `schema_snapshots.py --check` |
| A decimal written as a JSON number, with an exponent, or `null` where not nullable | Golden comparison, partly | Conformance test (`pattern`, `type`) |
| An enumerated value the schema does not list | Nobody | Conformance test (`enum`, `const`) |
| A result claiming a method or envelope version whose snapshot differs | Nobody | Conformance test (snapshot named by the result) |
| The documented order of defaults applied (input order), outputs not computed (by pointer, indices as numbers), checks (by result block, then id), warnings (order of their echo in `explain.conventions`), errors (by pointer, code, message) | Golden cases at positions | A new invariant test over every golden result, in `test_golden_conformance.py` |

What neither catches: an order the documentation does not state, and values, which stay the
golden cases' job.

## 4. The migration

### 4.1 Script

`scripts/golden_values_migration.py`, on the model of `scripts/golden_envelope_migration.py`:

- `list` runs the engine of `main` on each case that has an envelope-layout key, reads which
  entry each positional key points at, and writes the key of that entry (`pointer`, `code` and
  `pointer`, `pointer` and `id`). The engine only names the entry; every expected value is
  copied verbatim from the golden file. Output: `golden-migration.json` (machine-readable
  plan) and `golden-migration.md` (the list for approval).
- `apply FILE...` rewrites only the files named, only when their SHA-256 is the one in the
  plan. Top-level blocks that do not change are kept byte for byte; inside a block that loses
  some keys, the remaining entries are kept byte for byte; new sections are appended.
- `verify` checks each migrated file against its base copy: every old assertion and its new
  form resolve to the same path of the same engine result and carry the same expected text;
  every kept block is unchanged; each new section equals the plan; nothing is dropped.

### 4.2 Numbers

119 files change (75 `statements.reorganise`, 33 `ratios.*`, 11 others); 1,437 keys move; no
expected value changes; no assertion is dropped; `expect` (value) and `input` are untouched in
every file; 547 files are not touched.

| Change | Keys |
| --- | --- |
| `expect_meta` -> `expect_defaults` | 607 |
| `expect_diagnostics` -> `expect_not_computed` | 441 |
| `expect_diagnostics` -> `expect_checks` | 321 |
| `expect_diagnostics` -> `expect_warnings` | 4 |
| `expect_error_report` (`/errors/N/...`) -> `expect_errors` | 30 |
| `expect_diagnostics_length` -> `expect_counts` | 21 |
| `expect_diagnostics_length` -> count inside `expect_not_computed` | 5 |
| `expect_length` -> `expect_counts` | 8 |

The exact file list, by folder and by file, is
[`golden-migration.md`](golden-migration.md); the plan is `golden-migration.json`.

### 4.3 Dry run

On a temporary copy of `tests/` (never on the repository):

```text
python scripts/golden_values_migration.py list --markdown specs/0013-golden-values-schema-shape/golden-migration.md
python scripts/golden_values_migration.py apply <the 119 files> --root /tmp/i351-dry
python scripts/golden_values_migration.py verify --root /tmp/i351-dry --base-root .
119 checked, 0 problems
```

Negative checks on the copy: an expected value changed (`not_defined` to `not_applicable`) is
reported; a key renamed to an entry that does not exist is reported; two default entries with
their values swapped (same values, other entries) are reported; a changed `tolerance` block is
reported. `list` run again on the migrated copy finds 0 files to change.

### 4.4 Order of work after approval

1. Tests first: the semantic sections, the `null` rule and the refusal of retired keys in
   `crates/fairmark-core/tests/golden.rs` and `python/tests/test_golden.py`, tested on
   temporary cases as the diagnostics keys were; tests of the script on temporary copies, as
   `test_golden_envelope_migration.py` does.
2. `test_golden_conformance.py` and the order invariants; `jsonschema` declared in `dev`.
3. The rule in `.agents/skills/golden-case/SKILL.md` (then `uv run just generate`) and a new
   `tests/README.md`.
4. `apply` on the 119 files the maintainer approves by name; `verify` against `origin/main`.

## 5. Estimates for #266 and #263

Counted by script on the 666 files of `main`. "Before" is today's format, "after" the format of
section 2. Input changes cannot be avoided by any golden format: they are listed apart.

### #266: null equals absent, one number grammar

The issue admits two readings; both are counted.

| Effect | Before | After |
| --- | --- | --- |
| Input with an explicit `null`: `dcf.fcff/error-explicit-null-timing.yaml` (an error today, a default under "null equals absent") and `statements.reorganise/adjustment-dividends-declared.yaml` (`null` inside an amounts list) | 2 | 2 |
| Inputs with JSON numbers that have a fraction ("no floats in money paths") | 0 | 0 |
| Inputs with Italian number formats (`1.000,50`): none; today they fail with `FM-E001` `type` | 0 | 0 |
| Reading 1, input only: total | 2 | 2 |
| Reading 2, the result also omits a field instead of writing `null`: golden cases asserting `null` (277 keys in `expect`, 1 in `expect_error_report`) | 67 | 0 |
| Reading 2: total | 69 | 2 |

### #263: statements contract

`statements.reorganise` is at v2 since #361, so #263 produces v3. 156 golden files, 88 of them
value cases.

| Effect | Before | After |
| --- | --- | --- |
| Adjustment amounts keyed by date, flat adjustments: inputs with `adjustments` (input, unavoidable; 9 of them also assert `/adjustments/N` pointers) | 27 | 27 |
| Years sorted by the engine: `error-dates-newest-first.yaml` becomes a value case | 1 | 1 |
| An amounts unit with a documented default, reported in `meta.defaults_applied` (in input order): every positional default after it shifts | 22 | 0 |
| Write-down default fixed in the schema: no output change expected | 0 | 0 |
| Narrow reading, union of the above | 41 | 28 |
| Wide reading adds: a closing date field required for the 8 multi-year inputs, and a breaking change of the reclassified input (31 inputs) | +32 | +36 |
| Wide reading, union | 73 | 64 |

If the amounts unit is required rather than defaulted, every input changes (156 files) in both
formats.

### Retrospective: #361

Of the 131 files #361 touched, 53 would still change under section 2, for one or more of three
reasons: input fields renamed (13 files), fields of `value` renamed (17), pointer values that
changed form, `/x` becoming `/value/x` (20). 78 would not have changed. Counted from
`specs/0012-envelope-diagnostics/golden-migration.json`.

## 6. Open questions

1. **Paths inside `value`: plain pointers or selectors?** Recommended: plain RFC 6901 pointers
   relative to `value`; no selector syntax (section 2.3).
2. **Wording in existing and new cases.** The migration keeps the 47 wording assertions.
   Recommended: new cases may not assert `message`, `fix` or any `reason` text; existing ones
   stay until a change of wording touches them, then they are removed with the maintainer's
   approval rather than updated. If the closest-field suggestion of `fix` is contract, it
   should become a field (for example `suggestion`), not wording.
3. **`null` means null or absent, before #266.** Recommended: yes, in the same change as the
   sections, with the schema check of keys as the typo guard.
4. **Completeness of semantic sections.** Recommended: entries are "contains"; completeness is
   stated by `expect_counts`, required in new cases for `errors`, `warnings` and
   `not_computed` whenever entries of them are asserted; not required for `expect_defaults`,
   so a new documented default does not touch every case.
5. **Identity of an error entry.** `code` and `pointer` name every entry of the current golden
   set uniquely. Recommended: keep two parts; if a case ever needs two entries with the same
   code and pointer, extend the key with `reason` by a decision recorded in the README.
6. **Where the conformance test runs.** Recommended: Python only (section 3.3).
7. **Inner values of analysis results** validated against the inner method's snapshot.
   Recommended: yes; it is the one place where `value` holds another method's output.
8. **Plan files in the repository.** `golden-migration.json` is 548 KB. Recommended: commit it,
   as SPEC-0012 did, because `verify` and the approval refer to it; remove both plan files in
   the pull request after the migration if the maintainer prefers.
9. **Identifiers the schema does not enumerate (467 assertions).** Recommended: leave to #352;
   once enumerated they become checked by the conformance test as well.
10. **Spec.** Recommended: after the maintainer answers these questions, `spec-writer` turns
    this design into `spec.md` with acceptance criteria (runner semantics, conformance test,
    migration), so `test-engineer` can write the failing tests first.
