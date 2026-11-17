# SPEC-0012 — One envelope: diagnostics block and envelope version

| | |
| --- | --- |
| Status | Approved (maintainer, 2026-11-16; decisions 13 to 15 confirmed 2026-11-17) |
| Milestone | M3 · Agent layer and v0.1 release |
| Owner | @salvo8519 |
| Formula basis | No formula: a contract change that alters no computed number. Basis: ADR 0014 G2, G3, G4, G5, G8 and G10, the maintainer decisions of 2026-11-09 and those of 2026-11-16 below; RFC 6901 for every pointer; JSON Schema 2020-12 for the result schema. |
| Related | ADR 0014, ADR 0009, ADR 0012; issues #257, #258, #260, #263, #265, #266, #279, #285, #286; SPEC-0001, SPEC-0005, SPEC-0007, SPEC-0010 |
| Commit scope | Existing scopes: `explain` (envelope, diagnostics, warning catalogue in the core), `statements` and `ratios` (their version bumps), `analysis`, `py`, `cli`, `tools`, `mcp` (adapters), `adr`, `schemas`, `docs`, `i18n`. No new scope. |

## Problem
An AI agent that reads a Fairmark result must today parse each method in its own way to learn
what the result does not contain and why. `statements.reorganise` lists outputs not computed and
the checks of the statements inside `value`, per year; the six `ratios.*` methods list outputs not
computed inside `value` and give the reason a ratio is not defined as free text in the ratio;
every warning is English text in `explain.conventions` that starts with `warning: `; the defaults
applied name their field `field`; the envelope has no version. ADR 0014 G4 fixes one envelope for
every method, with a `diagnostics` block for what the result does not contain (`not_computed`),
what the reader could miss (`warnings`, with stable codes) and what was verified (`checks`), and
`meta.envelope_version`. This spec makes that envelope the contract of every method before v0.1,
when no contract has reached a user.

## Scope
**In scope:**
- The envelope of ADR 0014 G4, for every registered method, with `meta.envelope_version` = 1.
- Moving `not_computed` and `checks` out of `value` into `diagnostics` (`statements.reorganise`,
  the six `ratios.*` methods), and the reason of a ratio that is not defined into
  `diagnostics.not_computed` with `reason_code` `not_defined`, with a version bump of each of
  these seven methods in the same change.
- The pointer fields of the analyses renamed to the G4 grammar on the input and the output
  side, with a version bump of `analysis.goal_seek`, `analysis.scenarios`,
  `analysis.sensitivity` and `analysis.tornado` (decision 13 of 2026-11-17).
- Every `warning: ` convention, the rounding warning of #279 included, becomes a coded entry of
  `diagnostics.warnings`; the analysis preview convention gets a code too. Catalogue page
  `docs/en/reference/warnings.md` (and its Italian edition) and a test that every emitted warning
  has a documented code.
- `meta.defaults_applied` entries as `{pointer, value, reason}`, always present.
- The pointer rules of G4, with the amendment admitting `*_pointer` (maintainer decision 7).
- Every adapter (CLI, Python SDK, function-calling tools, MCP) passes the envelope through
  unchanged; the Python `Result` exposes `diagnostics`.
- Schema snapshots (`schemas/envelope/v1.json`, the new method versions) and the golden cases
  whose expected keys move.

**Out of scope:**
- New warnings for conditions that are not warned today (for example #216, the plausibility
  warnings of G10.5, the `growth_cap` reports of `tv.*`): each is added by the spec of its
  method, with the next free code.
- The version pin and the MCP `_meta` method version (#260).
- Null equals absent and the number grammar (#266); the statements v2 input contract (#263).
  Each bumps the versions it changes again (maintainer decision 12).
- Any change to a computed number, to an error code or to the error report (G5, #258).

## Maintainer decisions of 2026-11-16
Recorded from the maintainer's review of the draft (22:29 and 23:51 Europe/Rome).

1. **Empty diagnostics.** `diagnostics.not_computed`, `warnings` and `checks`, and
   `meta.defaults_applied`, are always present, `[]` when empty, and required in the schema.
2. **`explain.method` removed.** `explain` is `{formula, steps, conventions}`; the removal is an
   envelope change, covered by `envelope_version`, not by method versions.
3. **Warning codes** are `FM-W001` onward, never reused or renumbered.
4. **Checks of a year** carry `pointer` to the result block of the year (`/value/years/<i>`).
5. **Ratios** keep the `RatioValue` object with `value` only; the reason of a ratio not defined
   is in `diagnostics.not_computed`.
6. **Echo.** Every warning is echoed in `explain.conventions` as `warning FM-Wnnn: <message>`.
7. **Singular pointer fields.** A field holding one pointer may end in `_pointer`. This amends
   ADR 0014 G4, recorded there as a maintainer decision of 2026-11-16.
8. **Analysis preview.** The convention that an evaluated method is preview gets a warning code
   (FM-W016).
9. **Nested evaluations** are reported once: the warnings of the evaluation of `base_input`, and
   for `analysis.goal_seek` the diagnostics of the solved evaluation under the pointer of the
   inner value; the cells of a table keep their compact errors.
10. **Envelope snapshot** `schemas/envelope/v1.json` with an `envelope` entry in
    `schemas/index.json`.
11. **Scope** matches issue #259; its comment adds the catalogue page and the test that every
    emitted warning has a documented code, and includes the rounding warning of #279.
12. **No phased versioning.** #259 moves `not_computed`, `not_defined` and `checks` out of
    `value` now and bumps every affected method in the same change. #263 and #266 bump again
    where they change a contract; extra pre-release versions are accepted because nothing is
    released.

## Decisions of 2026-11-17
Taken by the orchestrator under the maintainer's mandate of 2026-11-17 ("one rule, no
exceptions; no number changes"), on the questions raised by T1 and T3.

13. **Analysis pointer fields follow G4, on every side.** AC10 has no exemption. The fields of
    the analyses that hold pointers are renamed on the input and the output side alike:
    `analysis.goal_seek` input `output` → `output_pointer`, `variable` → `variable_pointer`;
    `analysis.scenarios` input `outputs` → `output_pointers`, output `BaseOutput.output` and
    `ScenarioOutput.output` → `output_pointer`; `analysis.sensitivity` input and output
    `output` → `output_pointer`, `Axis.variable` → `variable_pointer` (the axis is echoed in
    the output); `analysis.tornado` input `output` → `output_pointer`,
    `TornadoVariable.variable` → `variable_pointer`, output `output` → `output_pointer`,
    `TornadoBar.variable` → `variable_pointer`. Each of the four methods goes from v1 to v2 in
    this change (breaking input and output changes; extra pre-release versions are accepted,
    decision 12). `analysis.goal_seek` and the input `outputs` of `analysis.scenarios` were not
    named in the decision: they follow from its rule (AC10 on every input schema).
14. **Missing input before not defined.** A diagnostics entry is one per output (R5). When an
    output is both not defined and missing an input, its single entry has `reason_code`
    `missing_input`, with the missing pointers of today in `missing_any_of`: the missing input
    is the actionable cause, and an output not computed for lack of input has no defined or
    undefined status. The `not_defined` text of that output is dropped.
15. **Binary floats reported by the engine (FM-W017).** The MCP server appended its own
    convention `warning: ... arrived as a binary floating-point number` to the envelope, a
    seventeenth condition the inventory of the core did not list. Under the rules "the engine
    decides, adapters only transport" and "one rule, no exception": the condition becomes warning
    FM-W017, emitted by the core through the warning API like every other. An adapter that
    receives arguments already parsed by a client library (function calling with a dict, the MCP
    server, LangChain, all through `fairmark.tools.dispatch`) passes the input pointers of the
    values that arrived as Python floats as a run option (`RunOptions.binary_floats` of the core,
    the `binary_floats` argument of the PyO3 `run` and of `fairmark.run_json`), not as an input
    field: no method schema and no `meta.input_hash` changes. The adapter never writes to the
    envelope.

## Inventory of the current code (main at the start of #259)
Read from `crates/fairmark-core/src`. Task T1 of `tasks.md` re-runs the inventory with a script
and adds any site missed here before the code changes.

### Warnings emitted as `warning: ` conventions, and the preview convention
Fifteen `warning: ` emission sites in production code (the other occurrences of the string are
tests and the `/base_input` filter of goal seek in `registry.rs`), plus the preview convention of
the analyses, plus the note the MCP server wrote for a value that arrived as a binary float
(`python/fairmark/mcp_server.py`, found during T16, FM-W017 by decision 15). One code per condition whatever the method. Pointer root: `input` is the root of
the input the client sent (`/base_input/...` inside an analysis); `envelope` is the root of the
result.

| Code | File | Condition | Methods | Pointer (root) | Message pattern (as today, without the prefix) |
| --- | --- | --- | --- | --- | --- |
| FM-W001 | `reading.rs` (called from `registry.rs` `execute`) | A decimal of the input has more digits than a decimal holds and is rounded as it is read (G7, #279) | every method | the field (input) | `` `{pointer}` has more digits than a decimal holds: given {text}, used {used}, rounded half away from zero ... `` |
| FM-W002 | `series.rs` | The base value is negative | `series.project` | `/base_value` (input) | `the base value {v} is negative, so a positive growth makes the values more negative; check the sign of the series` |
| FM-W003 | `dcf/engine.rs` `project` | A growth is applied to a negative flow, once per period | `dcf.fcff`, `dcf.fcfe` | `/stages/{i}` of the generated stage (input) | `period {k}: growth {g} applied to a negative flow ({prev}); check the sign of the projection` |
| FM-W004 | `dcf/engine.rs` `restate_growth` | Nominal perpetual growth below the expected inflation of the terminal phase | `dcf.fcff`, `dcf.fcfe` | `/terminal/{id}/growth` or `/terminal/tv.h_model/long_term_growth` (input) | `the perpetual growth {g} (nominal) is below the expected inflation of the terminal phase {pi}: the flows decline in real terms after the horizon (real growth {r})` |
| FM-W005 | `dcf/terminal.rs` `no_terminal_value` | No terminal value (`none`, given or by default) | `dcf.fcff`, `dcf.fcfe` | `/terminal` (input) | `no terminal value is included: the value covers the {n} explicit flows only; ...` |
| FM-W006 | `dcf/terminal.rs` `value` | A given terminal value is negative | `dcf.fcff`, `dcf.fcfe` | `/terminal/given/value` (input) | `the given terminal value {v} is negative (for example a decommissioning cost); it is discounted as given` |
| FM-W007 | `dcf/terminal.rs` `value` | The terminal value is negative, so its weight is not meaningful | `dcf.fcff`, `dcf.fcfe` | `/value/terminal_value_weight` (envelope) | `the terminal value is negative, so the terminal value weight is not meaningful as a share of the value` |
| FM-W008 | `dcf/terminal.rs` `report_technique` | Growth of the last explicit period differs from the perpetual growth (R11 of SPEC-0001) | `dcf.fcff`, `dcf.fcfe` | the growth field of the technique (input) | `the growth of the last explicit period ({a}) differs from the perpetual growth of the terminal value ({g})` |
| FM-W009 | `dcf/terminal.rs` `discount` | A price at a date (exit multiple, liquidation, residual value, given value) is discounted at N − 0.5 | `dcf.fcff`, `dcf.fcfe` | `/terminal_discount_time` (input) | `{what}; discounting it at N − 0.5 brings it forward half a period; it is discounted at N − 0.5 as chosen` |
| FM-W010 | `dcf/terminal.rs` `discount` | Under mid-period timing a perpetuity-growth terminal value is discounted at N | `dcf.fcff`, `dcf.fcfe` | `/terminal_discount_time` (input) | `under mid-period timing a perpetuity-growth terminal value is commonly discounted at N − 0.5, ...; it is discounted at the horizon N as chosen` |
| FM-W011 | `dcf/bridge.rs` `warn_if_negative` | The equity value is negative | `dcf.fcff`, `dcf.fcfe` | `/value/equity_value` (envelope) | `the equity value is negative ({v}): {reason}; it is reported as computed, not floored at zero` |
| FM-W012 | `dcf/fcff.rs` `bridge` | No net financial position given: zero is subtracted (maintainer decision 2 of 2026-11-09) | `dcf.fcff` | `/net_financial_position` (input) | `no net financial position given (...): zero is subtracted, so the equity value equals the enterprise value{rest}; ...` |
| FM-W013 | `ratios/mod.rs` `warn_defaults` | `other_items` of a net financial position not given: the default `[]` decides the net financial position (SPEC-0007 R13) | `ratios.capital_structure`, `ratios.debt_service` (the two callers of `warn_defaults`, T1) | `/{closing,opening}/net_financial_position/other_items` (input) | `no other items of the net financial position given (...): the net financial position is gross debt less cash; ...` |
| FM-W014 | `ratios/debt_service.rs` | `reserve_balance` not given: the LLCR assumes no reserve account | `ratios.debt_service` | `/debt_schedule/reserve_balance` (input) | `no reserve balance given (...): nothing is added to the present value of CFADS, ...` |
| FM-W015 | `statements/income_statement.rs` `write_down_warnings` | A non-zero B10 c or B10 d is placed below EBITDA by the default treatment `operating` (G3 test 4) | `statements.reorganise` | `/treatments/write_downs_of_fixed_assets` or `/treatments/write_downs_of_receivables` (input) | `year {name}: {item} ({v}) is in {line}, below EBITDA, by the default ...: it lowers operating_profit; ...` |
| FM-W016 | the analyses (convention "Evaluated method ... is preview", ADR 0012) | The method an analysis evaluates is `preview` | `analysis.*` | `/method` (input) | `Evaluated method {id} is preview: ...` (text of today) |
| FM-W017 | `registry.rs` `execute`, from the run option of the adapter (decision 15); formerly written by `mcp_server.py` `call_tool` | A value of the input arrived at the adapter as a binary floating-point number | every method, through function calling with a dict, the MCP server and LangChain | the field (input) | `` `{pointer}` arrived as a binary floating-point number ({value}): a JSON number may have lost digits before Fairmark read it; ... `` |

### Outputs not computed and checks inside `value`
| Method (current version) | Field today | Shape today | Moves to |
| --- | --- | --- | --- |
| `statements.reorganise` (v1) | `/years/{i}/not_computed/balance_sheet`, `/income_statement`, `/current_non_current` | `{output, missing, permitted[{treatment, destinations}], reason?}` | `diagnostics.not_computed` |
| `statements.reorganise` (v1) | `/years/{i}/checks/{relation id}` | `{status, pointers}`, status `reconciled` or `not_checked` | `diagnostics.checks` |
| `ratios.profitability`, `ratios.capital_structure`, `ratios.liquidity`, `ratios.working_capital`, `ratios.debt_service`, `ratios.covenants` (v2 each) | `/not_computed` | `{output, missing, reason}` (built by `NotComputedList`) | `diagnostics.not_computed` |
| the same six methods | `not_defined` of every `RatioValue` (for example `/roe/not_defined`, `/dscr/{i}/not_defined`, `/covenants/{i}/headroom_relative/not_defined`) | a sentence, or null | `diagnostics.not_computed` with `reason_code` `not_defined` |

The relation ids of the statements are those of `statements/checks.rs` (20 on the balance sheet,
10 on the income statement), `interest_cost_within_personnel_costs`
(`statements/income_statement.rs`) and `funds_invested_equals_sources` (`balance_sheet.rs`); T1
adds those of the reclassified form. The outcome fields of `ratios.covenants`
(`not_tested_code`, `not_tested_reason`) are results of the test, not diagnostics, and stay in
`value`.

No other method's output moves a list. The `value` of `analysis.goal_seek` (`inner_value`,
`solved_input`) follows the schema of the inner method through `x-fairmark-output-of` and
`x-fairmark-input-of`, so it moves nothing itself; the removal of `explain.method` and the
other envelope changes are covered by `envelope_version` (decision 2). T1 confirms this with a
scan of every output schema for `not_computed`, `checks` and `not_defined`. The analyses change
version for the renames of decision 13 only.

Fields inside `value` that hold pointers and are not named per G4 (statements only, verified):
`Assignment.aggregate`, `AppliedAdjustment.source`, `NfpComponent.source`,
`ReconciliationLine.sources`. The fields of the analyses that hold pointers are renamed by
decision 13. The schema test of AC10 finds any other, in every input and result schema.

### Inventory script (T1)
`scripts/envelope_inventory.py` re-ran the inventory on 2026-11-17 (head `a3c8c70`). It confirms:
fifteen `warning: ` emission sites and the preview convention, one site per code FM-W001 to
FM-W016 (the literal in `registry.rs` `rounded_on_reading` is the goal-seek filter, not an
emission); thirteen production calls of `DefaultApplied::new` or `destination`, those of the
defaults table below; `not_computed`, `checks` or `not_defined` in the current output schema of
exactly the seven methods that move lists and of no other method (`analysis.goal_seek`
included); the four statements pointer fields of R12. Differences, now
written into the tables of this section:

- FM-W013 is emitted by `ratios.capital_structure` and `ratios.debt_service` only, the two
  callers of `warn_defaults`.
- Relation ids not named above, seen in the golden results: statutory form
  `intangible_assets_net`, `tangible_assets_net`, `financial_fixed_assets_split`,
  `shareholder_loans_split`, `shareholder_loans_split_beyond_one_year`,
  `shareholder_loans_split_parts_beyond_one_year` (`balance_sheet.rs`) and
  `profit_before_tax_total`; reclassified form `operating_working_capital_total`,
  `invested_capital_total`, `total_funds_invested_total`, `net_financial_position_total`,
  `net_financial_position_maturity`, `total_sources_total`, `funds_invested_equals_sources`
  (`reclassified.rs`) and `value_added_total`, `ebitda_total`, `operating_profit_total`,
  `profit_before_financing_and_taxes_total`, `profit_before_tax_total`, `net_income_total`
  (`income_statement.rs`).
- Output fields of methods not bumped that hold pointers and are not named per G4:
  `analysis.scenarios` `BaseOutput.output` and `ScenarioOutput.output`, `analysis.sensitivity`
  `output` and `Axis.variable` (their descriptions say pointer), and `analysis.tornado` `output`
  and `TornadoBar.variable` (pointers whose descriptions do not say so); on the input side
  `analysis.goal_seek` `output` and `variable`, `analysis.scenarios` `outputs`,
  `analysis.sensitivity` `output` and `Axis.variable`, `analysis.tornado` `output` and
  `TornadoVariable.variable`. Settled by decision 13: all renamed, the four analyses bumped.
  No input schema of any other method has a pointer field outside the G4 grammar.

### Defaults applied (`meta.defaults_applied`, key `field` today)
| Method | Pointer | Value | Warned today |
| --- | --- | --- | --- |
| `dcf.fcff`, `dcf.fcfe` | `/stages/{i}/projection` | `"given"` | no |
| `dcf.fcff`, `dcf.fcfe` | `/stages/{i}/periods` | number of flows or lines | no |
| `dcf.fcff`, `dcf.fcfe` | `/cash_flow_terms`, `/discount_rate_terms` | `"nominal"` | no |
| `dcf.fcff`, `dcf.fcfe` | `/timing` | `"end_of_period"` | no |
| `dcf.fcff`, `dcf.fcfe` | `/terminal` | `{"none": {}}` | FM-W005 |
| `dcf.fcff` | `/net_financial_position` | `"0"` | FM-W012 |
| `statements.reorganise` | `/years/{i}/statutory/balance_sheet/{key}` (default destination of an aggregate given) | a destination id | no |
| `statements.reorganise` | `/years/{i}/statutory/income_statement/{key}` (A5, B14, B10 c, B10 d while their treatment is not given) | a destination id | FM-W015 for B10 c and B10 d |
| `statements.reorganise` | `/treatments/write_downs_of_receivables`, `/treatments/write_downs_of_fixed_assets` | `"operating"` | FM-W015 |
| `statements.reorganise` | `/years/{i}/statutory/balance_sheet/{key}` (item taken as zero, D1, D2) | `"0"` | no |
| `ratios.capital_structure`, `ratios.debt_service` (the methods reading a net financial position, T1) | `/{closing,opening}/net_financial_position/other_items` | `[]` | FM-W013 |
| `ratios.debt_service` | `/debt_schedule/reserve_balance` | `"0"` | FM-W014 |

Every other method keeps the default `apply_defaults` of the `Method` trait.

## Target envelope (ADR 0014 G4)
```
{ method, value,
  explain     { formula, steps, conventions },
  diagnostics { not_computed, warnings, checks },
  meta        { engine_version, envelope_version, method_version, method_status,
                input_hash, arithmetic, defaults_applied } }
```
Keys are serialised in this order (`meta.evaluated_method_status`, analyses only, after
`method_status`). Decimals are strings (G7). In the result schema every field below is
`required` unless "optional" is stated.

| Field | Type | Units and content | When present |
| --- | --- | --- | --- |
| `method` | string | Method id, for example `dcf.fcff`. | Always. |
| `value` | object | The method's output, described by `schemas/<method>/v<N>.output.json`. Nothing that is a diagnostic lives here (G4). | Always. |
| `explain.formula` | string | The method's formula in plain notation. | Always. |
| `explain.steps[]` | `{label, formula, value}` | Every step with numbers substituted; `value` a decimal string. | Always. |
| `explain.conventions[]` | string | Human text: conventions, defaults (`default: ...`) and the echo of each warning (R9). | Always, at least one entry. |
| `diagnostics.not_computed[]` | `{pointer, reason_code, reason, missing_any_of, permitted}` | `pointer`: the output not computed, from the envelope root (`/value/...`). `reason_code`: enum `missing_input`, `choice_required`, `not_defined`, `not_applicable` (values added, never renamed). `reason`: one sentence, never null. `missing_any_of`: list of alternative sets of input pointers (input root), any one of which makes the output computable; `[]` for `not_defined` and `not_applicable`. `permitted[]`: `{pointer, values}`, a choice the analyst must make (input pointer, permitted values); `[]` otherwise. | Always; `[]` when every output is computed. |
| `diagnostics.warnings[]` | `{code, pointer, message}` | `code`: string matching `^FM-W[0-9]{3}$`, documented in `docs/en/reference/warnings.md`. `pointer`: the field the warning concerns, from the root its catalogue entry states (input root, or envelope root starting with `/value`); `null` only when the catalogue entry says so. `message`: one sentence in English, guidance, not contract. | Always; `[]` when nothing is warned. |
| `diagnostics.checks[]` | `{id, status, pointer, pointers}` | `id`: the relation id, snake_case, documented on the method page. `status`: enum `reconciled`, `not_checked` (values added, never renamed; a relation that fails is an error, `FM-E026`, never a check). `pointer`: the result block the check belongs to, from the envelope root (`/value/years/<i>`). `pointers`: the input pointers of both sides, as read; `[]` when the sides are computed outputs. | Always; `[]` for a method without checks. |
| `meta.engine_version` | string | Version of the engine. | Always. |
| `meta.envelope_version` | integer | 1 for this envelope. | Always. |
| `meta.method_version` | string | Method version, a positive integer as a string (ADR 0009). | Always. |
| `meta.method_status` | string enum | `available` or `preview` (ADR 0012). | Always. |
| `meta.evaluated_method_status` | string enum | Analyses only: status of the evaluated method (ADR 0012). | Optional, analyses only. |
| `meta.input_hash` | string | SHA-256 of the canonical input with its defaults filled in; unchanged by this spec. | Always. |
| `meta.arithmetic` | string | The arithmetic used. | Always. |
| `meta.defaults_applied[]` | `{pointer, value, reason}` | `pointer`: the input field filled in, or the item given whose default destination applies (input root). `value`: the value as it would be written in the input (any JSON). `reason`: one line citing the practice or the rule. | Always; `[]` when no default applies. |

## Requirements (EARS)
- **R1** — WHEN any registered method returns a result, THE ENGINE SHALL return the envelope of
  "Target envelope", with exactly the top-level keys `method`, `value`, `explain`, `diagnostics`
  and `meta`, in that order.
- **R2** — THE ENGINE SHALL write `explain` as `{formula, steps, conventions}`, without
  `explain.method` (decision 2).
- **R3** — THE ENGINE SHALL always write `diagnostics` with its three arrays and
  `meta.defaults_applied`, each `[]` when empty, and the result schema SHALL list them as
  required (decision 1).
- **R4** — THE ENGINE SHALL write `meta.envelope_version` as the integer 1. WHEN `meta`,
  `explain` or `diagnostics` change after v0.1, THE PROJECT SHALL follow G8 against the envelope
  version, not the method version.
- **R5** — WHEN an output is not computed, THE ENGINE SHALL list it once in
  `diagnostics.not_computed` with the shape of "Target envelope": `missing_input` when inputs are
  missing (today's `missing` becomes the single alternative of `missing_any_of`),
  `choice_required` when a treatment or choice must be given (today's `permitted`
  `{treatment, destinations}` becomes `{pointer, values}`, and the choice is also in
  `missing_any_of`), `not_applicable` when no input could supply it in the form given (today a
  missing set that is empty, for example a statutory output of a reclassified entry), and
  `not_defined` for a ratio that a given term makes not defined. `reason` SHALL never be null. An
  output SHALL have one entry: WHEN an output is both not defined and missing an input, its
  entry SHALL have `reason_code` `missing_input`, and its `not_defined` text SHALL be dropped
  (decision 14).
- **R6** — THE ENGINE SHALL no longer write `not_computed` inside `value`
  (`statements.reorganise`, the six `ratios.*` methods), nor `not_defined` inside a ratio: a
  ratio is the object `{value}`, `value` null when not defined or not computed, and its reason
  is an entry of `diagnostics.not_computed` (G4, decision 5).
- **R7** — THE ENGINE SHALL no longer write `checks` inside `value` (`statements.reorganise`)
  and SHALL list every relation of every year in `diagnostics.checks`, with `pointer`
  `/value/years/<i>`, ordered by year, then by relation id (decision 4).
- **R8** — WHEN a condition of the warning catalogue occurs, THE ENGINE SHALL add one entry to
  `diagnostics.warnings` with its code, pointer and message; the seventeen conditions of the
  inventory take the codes FM-W001 to FM-W017 (FM-W017 by decision 15). A code is never reused or renumbered; a new
  condition takes the next free code (decision 3).
- **R9** — THE ENGINE SHALL echo every warning once in `explain.conventions`, as
  `warning FM-Wnnn: <message>`, in the same relative order as `diagnostics.warnings`; no
  convention SHALL start with `warning` without its entry in `diagnostics.warnings`
  (decision 6).
- **R10** — WHEN a documented default applies, THE ENGINE SHALL list it in
  `meta.defaults_applied` as `{pointer, value, reason}`; the key `field` is renamed `pointer`,
  and the `default: ` conventions keep their text.
- **R11** — THE ENGINE SHALL write every pointer of the envelope per RFC 6901 (`~0` for `~`,
  `~1` for `/`): a pointer into the result from the envelope root (`/value/...`, `/meta/...`);
  a pointer into the input from the root of the input the client sent (inside an analysis, under
  `/base_input`); the root of each warning pointer as its catalogue entry states.
- **R12** — THE ENGINE SHALL name every envelope field, and every input and output field of
  every method, that holds pointers `pointer` or `pointers`, or with a name ending in
  `_pointer` (one pointer) or `_pointers` (a list), except `missing_any_of` (G4 as amended,
  decision 7). In `statements.reorganise` v2: `Assignment.aggregate` → `aggregate_pointer`,
  `AppliedAdjustment.source` → `source_pointer`, `NfpComponent.source` → `source_pointer`,
  `ReconciliationLine.sources` → `source_pointers`. In the analyses v2, the renames of
  decision 13, on the input and the output side.
- **R13** — THE ENGINE SHALL order every diagnostics array deterministically: `not_computed` by
  `pointer` in the order of `ratios::pointer_cmp` (array indices as numbers); `warnings` in the
  order of their echo in `explain.conventions`; `checks` per R7. The same input and versions
  SHALL give the same diagnostics, byte for byte (constitution I).
- **R14** — WHEN an analysis evaluates another method, THE ENGINE SHALL report in its own
  `diagnostics` the warnings of the evaluation of `base_input` once, with input pointers prefixed
  by `/base_input`; `analysis.goal_seek` SHALL also report the `not_computed`, `warnings` and
  `checks` of the solved evaluation, result pointers rebased under `/value/inner_value`; the cells
  of a table keep their compact `{code, message}` errors (decision 9).
- **R15** — THE CLI (`fairmark run`), the Python SDK (`run_json`), the function-calling
  dispatcher and the MCP server SHALL return the envelope of the core unchanged. THE Python
  `Result` SHALL expose `diagnostics` (the block, decoded by the result schema) and the
  properties `not_computed`, `warnings`, `checks` and `defaults_applied`, each a list, which take
  precedence over a field of `value` with the same name. An adapter
  that receives arguments already parsed by a client library SHALL pass the pointers of the values
  that arrived as binary floats to the engine as a run option, and SHALL NOT write to the envelope
  (decision 15).
- **R16** — THE PROJECT SHALL keep `docs/en/reference/warnings.md` and its Italian edition, one
  row per code: code, condition, methods, pointer root, message pattern, the release that added
  it. A test SHALL fail when a code emitted by the engine is not in the catalogue, when a
  catalogue code has no emission site, or when a production source file writes a convention
  starting with `warning` other than through the warning API of the core.
- **R17** — THE PROJECT SHALL snapshot the envelope as `schemas/envelope/v1.json` with an
  `envelope` entry in `schemas/index.json`, immutable like the method snapshots (decision 10).
- **R18** — THE PROJECT SHALL, in the same change that moves the lists out of `value`, bump the
  version of every method whose input or output changes shape: `statements.reorganise` v1 → v2;
  `ratios.profitability`, `ratios.capital_structure`, `ratios.liquidity`,
  `ratios.working_capital`, `ratios.debt_service` and `ratios.covenants` v2 → v3; and, for the
  renames of decision 13, `analysis.goal_seek`, `analysis.scenarios`, `analysis.sensitivity`
  and `analysis.tornado` v1 → v2; each with `schemas/<method>/v<N>.json` and
  `v<N>.output.json` and its entry in `schemas/index.json`. No list is written both in `value`
  and in `diagnostics` at any commit on main. Earlier snapshots are never edited. #263 and #266
  bump these methods again when they change their contracts (decision 12). No other method
  changes version: the envelope changes are covered by `envelope_version`.
- **R19** — THE ENGINE SHALL keep the floor of G10 and the error report of G5 unchanged: a
  diagnostic is never an error, an error is never a diagnostic, and no computed number changes.
- **R20** — THE PROJECT SHALL record decision 7 as an amendment of ADR 0014 G4 (maintainer
  decision of 2026-11-16), so that the ADR and this spec state the same naming rule.

## Conventions and parameters
No convention of this spec changes a number. The contract choices it fixes:

| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Empty diagnostics arrays | always present `[]`; omitted when empty | always present | ADR 0014 G4; decision 1 |
| Warning code | `FM-W` + 3 digits | `FM-W` + 3 digits | scheme of `FM-E` codes (ADR 0013); decision 3 |
| Root of a warning pointer | stated per code in the catalogue | per code | ADR 0014 G4 pointer rule |
| Echo of warnings in conventions | `warning FM-Wnnn: <message>` | echo with code | ADR 0014 G4 ("may echo"); decision 6 |
| `not_computed` order | by pointer (`pointer_cmp`) | by pointer | SPEC-0007 R7 order, extended |
| `checks` locator | `pointer` to the result block | `pointer` | decision 4 |

## Computability, defaults and checks
The rows are the envelope fields; every method keeps its own table (SPEC-0001, SPEC-0005,
SPEC-0007), unchanged by this spec.

| Output | Inputs it needs | When an input is missing | Checks it takes part in |
| --- | --- | --- | --- |
| `diagnostics.not_computed` | the method's computability table | never missing: `[]` when every output is computed | none |
| `diagnostics.warnings` | the conditions of the catalogue | never missing: `[]` when no condition occurs | none |
| `diagnostics.checks` | the relations of the method (statements: R7, R12 of SPEC-0005) | never missing: a relation with a side missing is `not_checked`; a documented default never enters a side (SPEC-0005) | the relation itself |
| `meta.defaults_applied` | the documented defaults of the method | never missing: `[]` when none applies | none |
| `meta.envelope_version` | none | constant 1 | none |

Minimum input (G10): unchanged for every method; this spec adds no field to any input.

## Acceptance criteria
- [x] **AC1** (R1, R4) — Property over every golden input of every method: the result has exactly
  the keys `method`, `value`, `explain`, `diagnostics`, `meta` in order, and
  `meta.envelope_version` is the integer 1.
- [x] **AC2** (R2) — Unit test: no result has `explain.method`; `explain` has exactly `formula`,
  `steps`, `conventions`.
- [x] **AC3** (R3) — `tvm.npv` on `{"rate": "0.1", "cash_flows": [-100, 60, 60]}` returns
  `diagnostics` `{"not_computed": [], "warnings": [], "checks": []}` and
  `meta.defaults_applied` `[]`; the result schema of every method lists the four as required.
- [x] **AC4** (R5, R6) — `ratios.debt_service` with `balances: closing` and only
  `income_statement.ebit` and `interest` with `interest_definition`: `value` has no
  `not_computed`; `diagnostics.not_computed` has an entry `/value/nfp_to_ebitda` with
  `reason_code` `missing_input` and `missing_any_of` naming the missing EBITDA, its definition and
  the net financial position inputs, as today's list does.
- [x] **AC5** (R5, R6) — A `ratios.profitability` input with negative equity: `/value/roe/value` is
  null, `value.roe` has only `value`, and `diagnostics.not_computed` has `/value/roe` with
  `reason_code` `not_defined`, the reason of today and `missing_any_of` `[]`.
- [x] **AC6** (R5, R7, R12) — A `statements.reorganise` case with a missing treatment of B.III and a
  non-zero C15: the entry of each output C15 reaches has `reason_code` `choice_required` and
  `permitted` `[{"pointer": "/treatments/financial_fixed_assets", "values": ["operating",
  "non_operating", "financial_asset"]}]`; a reclassified entry lists a statutory-only output as
  `not_applicable`; `diagnostics.checks` lists every relation of every year with `pointer`
  `/value/years/<i>`; `value.years[i]` has neither `checks` nor `not_computed`, and its
  assignments carry `aggregate_pointer`.
- [x] **AC7** (R8, R9) — One unit test per catalogue code, FM-W001 to FM-W017, on the input of the
  existing test of that warning or convention: the entry has the code, the pointer of the
  catalogue and the message of today; the conventions contain `warning FM-Wnnn: <message>` once.
- [x] **AC8** (R8, R10) — `dcf.fcff` without `net_financial_position`: `meta.defaults_applied`
  contains `{"pointer": "/net_financial_position", "value": "0", "reason": ...}` and
  `diagnostics.warnings` contains FM-W012 at `/net_financial_position`; with
  `{"gross_debt": 0, "cash": 0}` neither appears (maintainer decision 2 of 2026-11-09).
- [x] **AC9** (R10) — No `meta.defaults_applied` entry of any golden result has a key `field`.
- [x] **AC10** (R11, R12, R20) — Schema test over every input schema and every result schema of
  every method, with no exemption (decision 13): every property that holds a JSON pointer (a
  string, or a list of strings, whose description says it is a JSON pointer) is named
  `pointer`, `pointers`, `*_pointer`, `*_pointers` or `missing_any_of`; a property that holds
  objects carrying pointers (for example `Scenario.overrides`) is not itself a pointer field.
  Property test over golden results: every envelope pointer resolves in the result
  (`/value/...`) or in the input sent, as its field states.
- [x] **AC11** (R13) — Two runs of every golden input give byte-identical `diagnostics`; reversing
  the order of the keys of the input changes nothing.
- [x] **AC12** (R14) — `analysis.sensitivity` of a `dcf.fcff` without a net financial position:
  FM-W012 appears once, at `/base_input/net_financial_position`; `analysis.goal_seek` on a
  `ratios.*` method with a ratio not computed reports it under `/value/inner_value/...`.
- [x] **AC13** (R15) — Cross-adapter test on one golden input per theme: the JSON of
  `fairmark run`, `fairmark.run_json`, the function-calling dispatcher and the MCP tool result
  equals the core result; `fairmark.run(...).diagnostics`, `.warnings`, `.not_computed`,
  `.checks` and `.defaults_applied` equal the corresponding JSON.
- [x] **AC14** (R16) — The catalogue test of R16 passes, and fails on a planted undocumented code and
  on a planted `warning:` literal outside the warning API.
- [x] **AC15** (R17, R18) — The schema check passes: `schemas/envelope/v1.json` exists and is
  listed; `statements.reorganise` v2, the six `ratios.*` v3 and the four `analysis.*` v2 have
  input and output snapshots and index entries; no current output schema has `not_computed`, `checks` or `not_defined`; the
  snapshots of earlier versions are byte-identical to `origin/main`.
- [x] **AC16** (R19) — Every `expect` value of every golden case, read at its new location, equals
  the value on `origin/main` (no numerical change); every `expect_error` case keeps its code and
  report.
- [x] **AC17** (R5, decision 14) — Unit test: `ratios.debt_service` without a debt schedule, and
  `ratios.profitability` with negative invested capital and no leverage inputs (the inputs of
  the golden cases `no-debt-schedule` and `negative-invested-capital`): `/value/llcr`,
  `/value/roe_leverage/roe` and `/value/roe_leverage/cost_of_debt` each have exactly one entry in
  `diagnostics.not_computed`, with `reason_code` `missing_input` and the missing pointers of
  today in `missing_any_of`; no entry with `reason_code` `not_defined` has their pointer.

## Golden cases
No expected number changes. Expected names change, so every case touched needs the maintainer's
named approval before it is edited (AGENTS.md; protected by `.claude/hooks/protect_golden.py`).
The script of task T3 (`scripts/golden_envelope_migration.py list`) prints the exact list of
files and keys; that list is shown to the maintainer before any golden file is touched. The
categories are:

- `tests/golden/statements.reorganise/*`: `expect` and `expect_length` keys under
  `/years/{i}/not_computed/...` and `/years/{i}/checks/...` move to the diagnostics; keys of the
  pointer fields renamed by R12 change; `expect_meta` keys `/defaults_applied/{k}/field` become
  `/defaults_applied/{k}/pointer`.
- `tests/golden/ratios.*/*` (six folders): `expect` and `expect_length` keys under
  `/not_computed` and every `/.../not_defined` key move to the diagnostics.
- Every case with an `expect_meta` key `/defaults_applied/{k}/field`.
- `analysis.*` cases whose `expect` reads `not_computed` or `not_defined` inside an inner value.
- `tests/golden/analysis.*/*` (decision 13): the input keys renamed (`output`, `variable`,
  `outputs` and the `variable` of an axis or of a tornado variable) and the `expect` keys of
  the renamed output fields (for example `/ranking/{i}/variable`); values unchanged.

The runners (Rust `crates/fairmark-core/tests/golden.rs` and Python) gain the optional keys
`expect_diagnostics` and `expect_diagnostics_length`, with pointers into `diagnostics`; adding
the keys is tooling, moving expectations into them is the golden change that needs approval.

## Interfaces
- Contract: follows ADR 0014 G1 to G10, with G4 amended by decision 7 (R20).
  `meta.evaluated_method_status` stays an optional field beside the G4 list (ADR 0012), and
  `diagnostics.checks[]` carries `pointer` (decision 4).
- Minimum input (floor, ADR 0014 G10): unchanged for every method; no new error code.
- Method ids: none added. Versions: `statements.reorganise` v2, the six `ratios.*` v3, and
  `analysis.goal_seek`, `analysis.scenarios`, `analysis.sensitivity`, `analysis.tornado` v2; no
  other method changes version.
- Input schema changes: the analysis pointer fields renamed (decision 13, breaking); the other
  new versions copy their inputs unchanged.
- Output changes: breaking for the bumped methods (fields removed from `value`, `not_defined`
  removed, pointer fields renamed, the analyses' pointer fields renamed); envelope version 1: `diagnostics` and
  `meta.envelope_version` added, `explain.method` removed, `meta.defaults_applied[].field`
  renamed `pointer` and always present.
- Documentation: `docs/en/reference/warnings.md` (new) and its Italian edition, the envelope
  description in the reference and in the agent skill, the methodology pages of the bumped
  methods, the Python API docstrings; a release-note fragment for analysts.

## Open questions
None: the questions of the draft were settled by the maintainer decisions of 2026-11-16.

Raised by T1 and T3 (2026-11-17) and settled by decisions 13 and 14 of 2026-11-17: the
analysis pointer fields and AC10 (decision 13); the three golden keys of an output both not
defined and missing an input (decision 14).

Raised during T16 (2026-11-17) and settled by decision 15 of 2026-11-17: the MCP note on values
that arrived as binary floats becomes warning FM-W017, emitted by the core from a run option.
