# ADR 0014 — Contract guidelines for inputs, the result envelope and errors

Status: proposed · 2026-11-09 (#257; maintainer decisions of 2026-11-09 recorded below)

## Context
Fairmark is used mainly by AI agents, which fill an input from a document, read the result
and correct their input from the error. The contract audit of 2026-11-09 (issue #257) found
that each method answers the same questions in its own way:

- **Missing data.** `dcf.fcff` takes an absent net financial position as zero and returns an
  equity value equal to the enterprise value, with nothing in `meta.defaults_applied`.
  `statements.reorganise` never takes a missing input as zero. The ratios require every
  amount, so an agent that lacks one tends to invent it.
- **What the result does not contain.** Only `statements.reorganise` lists outputs not
  computed, inside each year and without a reason; the ratios use `not_defined` strings;
  warnings are free text in `explain.conventions`; checks exist only for statements; pointer
  fields are called `field`, `pointer`, `output`, `missing`, `aggregate` or `pointers`.
- **Errors.** The report carries only `code` and `message`; the location is inside the
  English text; validation stops at the first error; `FM-E001` covers both schema errors and
  business-rule conflicts.
- **Shapes.** Four encodings of a choice (single-key object, externally tagged, internally
  tagged, untagged); labelled amounts named `label` + `value`, `name` + `amount` or
  `label` + `amount`; `periods` is a count, an array of lines or an object; adjustment
  amounts of the statements align with the years by position, and years without a date are
  taken as oldest first although Italian statements print the current year first.
- **Versions.** ADR 0009 bumps a method version on a numerical or breaking input change; it
  says nothing on outputs, and the envelope has no version.
- **Null.** `dcf.*` refuses `null` for defaulted fields and accepts it for others;
  `statements.reorganise` accepts it everywhere; the strict tool modes of several providers
  make every field required and express optional as nullable, so models send `null`.
- **Numbers.** The schema pattern refuses `"1e-5"`; the Python layer accepts it.

Every method version in `schemas/index.json` is `unreleased`: no contract has reached a user.
A breaking change now costs one method version and one snapshot; after v0.1 it costs a
deprecation. Three options were considered:

1. **Fix each method when its next spec touches it**, additively where possible. Rejected:
   the inconsistencies would ship in v0.1 and each later fix would be a deprecation.
2. **Guidelines plus one batched version bump per affected method before v0.1.** Chosen.
3. **Guidelines for new specs only**, existing methods unchanged. Rejected: agents would
   have to learn two contracts, and the `dcf.fcff` equity value would remain misleading.

## Decision
The guidelines G1 to G9 below are the contract of every method. Every new spec applies them;
`specs/_templates/spec.md` points to this ADR. A spec that departs from one states the
departure and the reason in its Interfaces section, for the maintainer's approval. No
principle of the constitution changes: G2 and G3 make principles II and III operational,
and the diagnostics are produced in the core, so adapters still only translate (IV).

### Maintainer decisions of 2026-11-09
1. **One batch before v0.1.** Every contract change, breaking ones included, is made before
   the v0.1 release, with one version bump per method: `statements.reorganise` v2,
   `dcf.fcff` v4, `dcf.fcfe` v2, `dcf.free_cash_flows` v3, and v2 of each `ratios.*` method
   whose input or output changes shape (the list is fixed by the implementing spec).
2. **No equity value without a net financial position.** In `dcf.fcff`, when
   `net_financial_position` is absent the equity value and the bridge's equity value are
   `null` and listed in `diagnostics.not_computed`. A company with no debt and no cash states
   it explicitly with zero amounts.
3. **Dated years.** A multi-year `statements.reorganise` input requires a closing `date` for
   each year; the engine sorts the years by date and reports the order in
   `explain.conventions`. Amounts that refer to years are keyed by date or id, not by
   position.
4. **Null equals absent, everywhere** (G3).
5. **A missing input is never zero**, unless a documented default applies, and every default
   applied is reported in `meta.defaults_applied` (G3).
6. **One envelope with a diagnostics block** and `meta.envelope_version` (G4).
7. **Structured errors**, and validation reports every error it finds (G5).
8. **Shapes that language models fill well** (G6), **one number grammar** (G7) and an
   **output versioning rule** (G8).

### G1. Minimal input first
Each spec states a minimal input and a maximal input, and the schema carries both as
`examples`. A field is required only when the method cannot produce any output without it.

### G2. Best effort
A method computes every output its inputs allow. An output it cannot compute is `null` and
has an entry in `diagnostics.not_computed`. Missing data is never an error and never a zero.
An error is raised only for an input that is present and invalid, or for a choice that the
method cannot make for the analyst and without which no output can be computed.

### G3. Not given, none, zero
Absent and `null` mean "not given", in every method and every field. `[]` means "none"; `0`
means zero. A defaulted field given as `null` takes its default. A default exists only when
the field description and the schema `default` keyword state it (the keyword is the real
default or is absent), and every default applied is listed in `meta.defaults_applied` as
`{pointer, value, reason}`, which replaces the current `field` key. An absent input never
enters a total as zero.

### G4. One envelope
Every method returns:

```
{ method, value,
  explain     { formula, steps, conventions },
  diagnostics { not_computed, warnings, checks },
  meta        { engine_version, envelope_version, method_version, method_status,
                input_hash, arithmetic, defaults_applied } }
```

- `not_computed[]`: `{pointer, reason_code, reason, missing_any_of, permitted}`. `pointer`
  locates the output in the envelope (`/value/...`); `reason_code` is a documented enum
  (initially `missing_input`, `choice_required`, `not_defined`, `not_applicable`; values are
  added, never renamed); `reason` is one sentence; `missing_any_of` is a list of alternative
  sets of input pointers, any one of which makes the output computable ("the total, or all
  its components"); `permitted[]` is `{pointer, values}` for a choice the analyst must make.
- `warnings[]`: `{code, pointer, message}`, with a stable, documented code.
- `checks[]`: `{id, status, pointers}`, with `status` from a documented enum.
- Nothing method-specific lives outside `value`. Ratios keep `value: null` for a ratio that
  is not defined and give the reason in `not_computed` with `reason_code: not_defined`.
- Pointers follow RFC 6901. A pointer into the result starts at the envelope root; a pointer
  into the input starts at the input root. Every field that holds pointers is named
  `pointer` or `pointers`, or ends in `_pointers`, except `missing_any_of`, which is defined
  here.
- `explain.conventions` stays human text and may echo defaults and warnings; agents read
  `diagnostics` and `meta`.
- `meta.envelope_version` is an integer, 1 for the envelope defined here. A change to `meta`,
  `explain` or `diagnostics` follows G8 against the envelope version, not the method version.

### G5. Errors
Every refusal is reported as `{code, message, pointer, pointers, reason, expected, fix}`:
`pointer` is the field to correct (`null` only for malformed JSON, where the message keeps
the line and column); `pointers` lists every field of a relation (both sides of an `FM-E026`
reconciliation, both fields of a conflict); `reason` is a sub-kind (`missing`,
`unknown_field`, `duplicate`, `type`, `enum`, `conflict`, `requires`, `sign`, `blank`,
`range`, `reconcile`); `expected` lists the accepted values where they are known; `fix` is
one sentence that resolves the error. `reason` separates the meanings now grouped under
`FM-E001` without new codes. Validation of the input reports every error it finds in
`errors[]`, with the first also at the top level; an error raised during the computation
stops it and is reported alone. A new meaning gets a new code; a new location keeps the code
of its condition (ADR 0013). Python, the CLI, MCP and function calling pass the whole report.

### G6. Shapes that language models fill well
Flat objects, at most four levels below the root. A choice among options is an enum field
(`kind`) plus optional fields validated at run time, not a `oneOf` of objects. No positional
alignment between arrays: entries are keyed by date or id. Enum values are `snake_case`
English, each with a description that tool definitions keep. One name per concept across
methods: `label` + `value` for a labelled amount, `periods` only for a count, `date` as ISO
8601 `YYYY-MM-DD`. Every input schema carries the examples of G1.

### G7. Units and one number grammar
Every description states the unit: currency amounts with their sign rule, rates as decimal
fractions per period with an example (0.09 = 9%), counts with their range. A decimal is a
JSON number or a string matching `^-?\d+(\.\d+)?([eE][+-]?\d+)?$`, in the schema, the core
and every adapter alike: decimal point, no thousands separators, no currency symbols.
Decimals are returned as strings.

### G8. Versioning of inputs and outputs
The method version is bumped on a numerical change, on a breaking input change, and on an
output field that is removed, renamed, retyped or becomes nullable. A new optional input
field and a new output field are additive. Input and output schemas are both snapshotted
(`schemas/<method>/v<N>.json` and `v<N>.output.json`); the immutability rules of ADR 0009
apply to both. The MCP `_meta` of each tool reports the method version beside its status.

### G9. Interoperability between methods
When a method produces what another consumes, the producing spec names the consuming field,
the type is reused rather than copied, and a golden case chains the two methods.

### Not decided here
A version pin supplied by the client (`expect_version`, with a new error code), the handling
of a Python `float` by the SDK, and a default for `balances` in the ratios are left to the
follow-up specs and to the maintainer.

## Consequences
+ An agent learns one contract: where a result says what it does not contain and why, which
  defaults were applied, and which field to correct, without parsing English.
+ No misleading number: an equity value is never computed on a net financial position that
  was not given.
+ Statements copied from a filing in its printed order are valued correctly.
+ The breaking changes cost nothing to users, because no contract has been released.
− Five or more method versions and their schemas are added in one batch, and every adapter
  changes its result and error decoding at once.
− Golden cases of `dcf.fcff` that omit the net financial position, and any golden case whose
  expected output changes shape, are protected: the approval of each version bump must name
  the cases it covers before they are added or changed.
− Run-time validation of `kind` fields replaces part of what `oneOf` expressed in the schema,
  so the core carries more validation code and tests.
− Earlier method versions keep their shapes; the guidelines bind only new versions.
