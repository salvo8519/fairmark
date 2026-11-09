# Error codes

Codes are stable: once released, a code never changes meaning. Every adapter (Python, CLI, MCP,
function calling) reports an error as the same report, described below: the code, the message
and the fields a client needs to correct the input without reading the message. Messages name
the field to correct by its JSON pointer (RFC 6901), for example `/stages/0/flows/1`, and state
the fix. For a method evaluated by an analysis, the message keeps the pointer within
`base_input`, while the `pointer` of the report starts at the root of the analysis input, under
`/base_input` (for example `/base_input/wacc`).

## The error report

A refused call returns `{"error": <report>}` (ADR 0014 G5):

```json
{"error": {
  "code": "FM-E001",
  "message": "invalid input: `principal` must be greater than 0 (got 0)",
  "pointer": "/principal",
  "pointers": [],
  "reason": "range",
  "expected": ["an amount greater than 0"],
  "fix": null,
  "errors": [
    {"code": "FM-E001", "message": "invalid input: `principal` must be greater than 0 (got 0)",
     "pointer": "/principal", "pointers": [], "reason": "range",
     "expected": ["an amount greater than 0"], "fix": null},
    {"code": "FM-E001",
     "message": "invalid input: `number_of_instalments` must be between 1 and 10000 (got 0)",
     "pointer": "/number_of_instalments", "pointers": [], "reason": "range",
     "expected": ["a whole number from 1 to 10000"], "fix": null}
  ]
}}
```

| Field | Content |
| --- | --- |
| `code` | The stable code of the table below. |
| `message` | What is wrong and, where known, how to fix it, in English. The same text as before the report gained the other fields. |
| `pointer` | JSON pointer of the field to correct, always from the root of the input you sent (inside an analysis, under `/base_input`); `""` (the empty pointer) for the input as a whole, for example an input that is not an object. It is `null` only for a text that is not JSON (`reason` `syntax`, the message gives the line and column), for an error raised during the computation that no single field causes (for example `FM-E005`, `FM-E008`, `FM-E012`), for `FM-E027`, which names its inputs in `missing_any_of`, and for `FM-E007` when the method id is not part of the input (the method argument of the CLI, of `fairmark.run` or of a tool call; inside an analysis it is `/method`). |
| `pointers` | Every field of a relation: both sides of an `FM-E026` reconciliation, both fields of a conflict, a value and the field its domain depends on (a nominal `/rate` and its `/compounding`, when given, or `/from`, a technique of another basis and `/basis`, `metric_value` and `metric` of an exit multiple), a missing field and the misspelt key that stands for it; empty when the error concerns one field. Every field that `pointer` and `pointers` name is in the input you sent, or is a field to give (`reason` `missing` or `requires`, for example `/inflation` of `FM-E024`, or an item an `FM-E026` reconciliation took as zero); a field left to its default (a `compounding` not given) is never named, and a conflict names only the fields given. |
| `reason` | The sub-kind of the error (values below), which tells apart the conditions that share a code, above all `FM-E001`; `null` when the code has one meaning only. |
| `expected` | A list of strings. When the accepted values form a closed list, the literal values: the values of an enumeration (`reason` `enum`, and the method ids of `FM-E007`), the fields an object accepts (`unknown_field`, the tag of an internally tagged object included), the keys of a choice written as an object with one key (`length`, `unknown_field`). Otherwise short descriptions of the accepted form in plain words, for example `"an amount greater than 0"` or `"a whole number of 0 or more"`, never a Rust type name. Empty when not known. When the message also quotes accepted values, `expected`, not the message, is the complete list. |
| `fix` | One sentence that resolves the error; `null` when the message is the only guidance. Its wording is guidance and may change between releases; rely on `code`, `pointer` and `reason`. |
| `missing_any_of` | `FM-E027` only: alternative sets of input pointers, any one of which lifts the input above the minimum of the method, for example `[["/closing/cash"], ["/closing/current_assets"]]`. |
| `errors` | Every error found, at least one (`minItems` 1); the first is the top-level report itself. The entries have no `errors` of their own. |

**Every input error, in one call.** Every method validates its whole input before it computes
anything: the validation goes on after a refusal and reports every error it finds, in the order
it checks them, so the first error is the one a single report would give. A check that needs a
refused input is skipped (for example the count of the rates when a DCF stage is refused, the
comparison of a growth with a rate that is itself refused, or the rest of an adjustment whose
amounts are not one per year); a check that does not is still made (a refused `wacc` does not
hide a refused terminal growth, a refused `principal` does not hide a rate out of range).

- **Reading the input.** Every field that cannot be read is reported: the first as `serde`
  reports it (ADR 0013), then every other value of the wrong type, missing required field and
  unknown value of an enumeration, each with the message, pointer, reason, `expected` and `fix`
  it would have if it were the only error. The text is also read as JSON and walked against
  the schema of the method, and every unknown key, every key given twice in one object and
  every object given as a positional array is reported, each with its pointer. An unknown key
  comes with the fields its object accepts in `expected` and a `fix` that names the closest
  accepted field when the key looks like a typing error, ignoring case (for example "rename
  `wac` to `wacc`"). `reason` `syntax` is given only for a text that is not JSON; a value of
  the wrong kind for a choice (`true` for `timing`, or an object for `formula`) is `enum` at
  its field with the values. A field has the same reason and `expected` wherever it is: inside
  a technique of `tv.compare` as in the technique given directly.
- **Reading errors and the checks of the method.** When the input cannot be read, the checks of
  the method still run on the fields that were read, and their errors are reported after the
  reading errors: a `"+0.1"` for one rate does not hide a tax rate of -1. Each refused value is
  replaced by one its schema accepts only so that the input can be read, and that value never
  decides an error that is reported. Every error depends on known fields: its `pointer`, its
  `pointers` and the fields its check otherwise read (for example the projection of a stage,
  which decides the fields the stage takes). It is reported only when none of these is a
  refused field, a field within one or a field that holds one: with `compounding` refused, the
  rule for `rate`, which depends on it, is not checked; with `stages/0/periods` refused, a
  `cost_of_equity` of -5 is still reported. A check never stops at a refusal before a check
  that does not depend on it.
- **A misspelt key stands for its field.** A key with a rename suggestion (`wac` for `wacc`)
  counts as the field it names: no check that depends on that field is reported, and when the
  field is required, its `missing` entry has `pointers` `[field, key]` (for example
  `["/wacc", "/wac"]`) and the same fix as the key, "rename `wac` to `wacc`".
- **One entry per condition.** An object given as an array is one `type` error at the object,
  with no error for its elements. A condition met more than once (a constant growth checked
  for every period) is reported once. When the tag of an object is refused (the `kind` of an
  adjustment of `statements.reorganise`, missing or unknown), the tag error is the only entry of
  that object, with its unknown keys: its other fields depend on the kind and are not guessed.
  An unknown key of such an object lists in `expected` the fields of every kind.
- **Number grammar.** A decimal is a JSON number or a string with the number grammar of ADR
  0014 G7, `^-?\d+(\.\d+)?([eE][+-]?\d+)?$`: decimal point, digits on both sides of it, an
  optional exponent with an optional sign (`"1E-7"`, `"1e+3"`), no other sign than a leading
  minus, no thousands separators and no currency symbols. A string outside it (`"+0.1"`,
  `".1"`, `"1."`, `"1_000"`, `"1,000"`) is `type` at its field, in every method. The engine
  accepts a signed exponent now; the pattern published in the input schemas still reads
  `[eE]\d+` and gains the sign with issue #266.
- **Ambiguous JSON is refused.** A key given twice in one object (`duplicate`, at the key) and
  an object given as an array of its values (`type`, at the array; `""` for an input that is an
  array) are refused in every method: they would be read with the last value or by position.
- **Choices written as an object with one key** (`terminal` of the DCF, the techniques of
  `tv.compare`, `growth` and `timeline` of `series.project`): a key that is not one of the
  choices is `unknown_field` at the key, with the choices and the closest one, also next to a
  valid key; when it is the only key (`{"tv.godon": {...}}`), that is the only entry, with no
  `length` for the choice it leaves empty. No key, or a second key that is one of the choices,
  is `length` (at the object, or at the second key), with the choices in `expected`.
- **Analyses.** The fields of the analysis and its `base_input` are checked in one pass:
  `base_input` is checked by the method it names, through the validation alone of that method
  (every check of its input, nothing computed), and its input errors are reported under
  `/base_input`, also when the analysis input itself cannot be read. Checking `base_input`
  adds no evaluation: an analysis evaluates the inner method only for its results.
  `analysis.goal_seek` checks `base_input` with the variable set at the low end of the
  bracket, the first value it evaluates, also when the analysis input itself cannot be read, so
  the value the variable replaces is never refused. A variable or an override that points to a
  field taking a whole number is `FM-E015` at its own field. A variable or an override that
  points to a value `base_input` refuses as it is read (a value of the wrong type, a missing
  field, a field a misspelt key stands for), or to a field within it, is not checked: the
  refusal is reported under `/base_input`, without an `FM-E015`. An unknown method (`FM-E007`) is
  the only error about it: the variables are not checked against it. The size of the request
  (`FM-E016`, an input error) is checked first: a request over the limits evaluates nothing.
  The inputs an analysis builds for its evaluations from a checked `base_input` are read by
  `serde` alone, not walked against the schema again.
- **Computation errors.** An error of the computation (an overflow, a solver that does not
  converge, a defect) is never listed next to input errors: when the input has errors, they
  are reported; only an input without errors reaches the computation, whose first error stops
  it and is the only entry.
- **Input conditions found by the computation.** A few conditions of the input can only be
  found by computing, so they are not part of the validation (nor of the check of
  `base_input` in an analysis) and are reported alone, once the input has no other error: a
  reconciliation of totals of `statements.reorganise` (`FM-E026`); a rounding too coarse for a
  loan schedule (`FM-E023`, at `/rounding/decimal_places`); for
  `loans.number_of_instalments`, an instalment that never repays the loan (`FM-E021`) and a
  count of instalments solved above the limit (`FM-E001`); a date generated by `dates.schedule`
  after 2199-12-31 (`FM-E009`); an input of a `ratios.*` method below its minimum
  (`FM-E027`).

The schema of the report is part of every result schema (`fairmark schema <method>
--envelope`, `fairmark.result_schema(method)`): `$defs/FairmarkErrorResponse`, named at the
root by `x-fairmark-error`. The cells of an analysis table keep their own compact report,
`{"code", "message"}`, which is part of the output of the analysis methods.

### Reasons

| `reason` | Meaning | Codes |
| --- | --- | --- |
| `syntax` | The text is not JSON; `pointer` is `null` | `FM-E001` |
| `missing` | A required field, or one required by the other fields given, is absent; for `FM-E027`, no output has every input it needs | `FM-E001`, `FM-E025`, `FM-E027` |
| `unknown_field` | A field the input does not define; `expected` lists the fields accepted | `FM-E001` |
| `duplicate` | A field given twice in the same object, or a repeated pointer of an analysis | `FM-E001` |
| `type` | A value of the wrong type, `null` for a field that refuses it, an input that is not an object, an object given as a positional array, or a date that is not a calendar date | `FM-E001`, `FM-E009` |
| `enum` | A value outside a closed list, or a value of another kind for a choice; `expected` lists the values; an unknown method (`expected` lists the method ids) | `FM-E001`, `FM-E007` |
| `conflict` | Fields that exclude each other are given together, or contradict each other (bases, dates out of order) | `FM-E001`, `FM-E017`, `FM-E019` |
| `requires` | A field is given without the field it needs (cash flows and discount rates in different terms without expected inflation), or is not used by the choice made | `FM-E001`, `FM-E020`, `FM-E022`, `FM-E024`, `FM-E025` |
| `sign` | An amount with the wrong sign for its field | `FM-E001`, `FM-E006` |
| `blank` | A text that must say something is empty or blank | `FM-E001` |
| `range` | A number outside the domain of its field | `FM-E001`, `FM-E003`, `FM-E004`, `FM-E009`, `FM-E010`, `FM-E018`, `FM-E020` to `FM-E023` |
| `length` | A list with the wrong number of entries (empty, too long, not one entry per period or year), or a choice written as an object with no key or with two | `FM-E001`, `FM-E002`, `FM-E025` |
| `reconcile` | Two sides of a relation that must agree differ | `FM-E026` |

Values are added, never renamed or removed: a client accepts a value it does not know. The
codes raised during the computation (`FM-E005`, `FM-E008`, `FM-E011` to `FM-E014`) have
`reason` `null`, as do `FM-E015` and `FM-E016`, which have one meaning each; `FM-E015` still
names the field that holds the analysis pointer (`/variable`, `/output`, `/outputs/k`, an
override `pointer`).

## Codes

| Code | Meaning | Typical fix |
| --- | --- | --- |
| `FM-E001` | Invalid input: wrong type, missing required field, unknown field, a field given twice, unknown value of a choice; the message names the field by its JSON pointer and keeps the values expected, for example ``invalid input at `/terminal/tv.gordon/first_flow`: missing required field `first_flow`; add it`` or ``invalid input at `/stages/0/projection`: unknown variant `constant`, expected one of `given`, `constant_growth`, `linear_decline` ``; only a malformed JSON text (a syntax error, a truncated input) gives the line and column instead. Also: in `dates.schedule` and the timelines of `series.project`, both of `frequency` and `period_months` (in `dates.schedule` also neither; the message names both pointers), a `period_months` of 0, or a `number_of_periods` outside 1 to 10000; in `series.project`, a growth rate of −1 or lower as given or after conversion (named by its pointer, for example `/growth/per_period/growth_rates/1`), a `per_period` list without one rate per growth step, a missing or inapplicable `growth_basis`, and a `timing` missing with a `schedule` timeline or given with a `periods` timeline | Correct the field the pointer names: add a missing field, remove or rename an unknown one, use one of the values listed; give exactly one of `frequency` and `period_months`; `fairmark schema <method>` gives the full input |
| `FM-E002` | Empty cash flow series | Provide at least one cash flow |
| `FM-E003` | Rate out of range: the rate per period, or per compounding period, is at or below −100%; the message names the rate by its JSON pointer, for example `/wacc_per_period/3` or `/inflation` in the DCF or `/finance_rate` in `tvm.mirr`; in the DCF a rate converted with expected inflation that reaches −100% (possible only at the edge of the decimal range) keeps the pointer of the rate as given and adds the inflation it was converted with; for a nominal rate with $m$ compounding periods a year (`rates.convert`, `tvm.xnpv`, `loans.*`) the message names the rate, for example `/rate`, and states the rule that applies: the rate per compounding period $r/m$ must be greater than −1, so the rate must be greater than $-m$; `tvm.irr` and `tvm.xirr` apply the same rule to `guess`, before any iteration, and name `/guess` | Rates are decimal fractions: 10% is `0.10`; a nominal rate with compounding is checked per compounding period ($r/m$) |
| `FM-E004` | Growth ≥ discount rate (terminal value, perpetuity); in the DCF and in `tvm.perpetuity` the message names the growth and the rate by their JSON pointers; in the DCF, when the terminal rate was converted with expected inflation, it also names the inflation field | The Gordon model needs g < r; for a converted terminal rate, also check the expected inflation |
| `FM-E005` | IRR or XIRR solver did not converge; the message names the solver | The rate may lie outside the decimal range or far from `guess`: give a `guess` near the expected rate |
| `FM-E006` | `cash_flows` has no positive or no negative amount | IRR, XIRR and MIRR need both outflows and inflows |
| `FM-E007` | Unknown method or tool | See `fairmark methods` |
| `FM-E008` | Arithmetic overflow | Inputs out of a realistic range |
| `FM-E009` | Invalid date: malformed, impossible (`2023-02-29`) or outside 1900-01-01 to 2199-12-31; a date read from the input is named by its JSON pointer, for example `/cash_flows/1/date`; a schedule (`dates.schedule`, or the `schedule` timeline of `series.project` at `/timeline/schedule/number_of_periods`) whose generated end date falls after 2199-12-31 is named at `/number_of_periods`, with the first date out of range and the last period that fits | Write dates as ISO 8601 `YYYY-MM-DD` within the supported range; for a schedule, give fewer periods or a shorter period length |
| `FM-E010` | A cash flow is dated before the valuation date | Move the valuation date to or before the earliest flow, or omit it to use the earliest flow date |
| `FM-E011` | No single rate can be isolated: more than one rate sets the present value to zero, a repeated root is possible, or the sign changes of the flows show that another rate exists; the message lists the intervals | The IRR is ambiguous: use `tvm.npv` at the cost of capital, or `tvm.mirr` with explicit rates |
| `FM-E012` | Internal error: a defect in Fairmark, not in the input | Report it with the `input_hash`; the input is never needed |
| `FM-E013` | Goal seek: the output minus the target has the same sign at both ends of the bracket | Widen the bracket so that the target lies between the outputs at its two ends |
| `FM-E014` | Goal seek did not reach the tolerance within `max_iterations`; also returned when the output jumps across the target (discontinuity) | Raise `max_iterations` or the tolerance; check that the output is continuous in the variable |
| `FM-E015` | An analysis pointer (`variable`, override `pointer`, `output`) does not resolve to a decimal | Use an RFC 6901 pointer to an existing decimal field, for example `/discount_rate` |
| `FM-E016` | The analysis needs more than 10,000 evaluations of the inner method, or more than 1,000,000 evaluations × values in `base_input` | Split the request into smaller tables, or shorten the inner input |
| `FM-E017` | An analysis method was asked to evaluate another analysis method | Run the inner analysis directly; analyses do not nest |
| `FM-E018` | A terminal value input is outside its economic domain: a return on new invested capital or a capitalisation rate of zero or below, a multiple or metric of zero or below, a finite horizon outside 1 to 10,000 periods, a negative half-life, or a negative asset value, liability, liquidation cost or tax on gains | Check the sign and the unit of the field named in the message; obligations belong in `liabilities`, not as negative assets |
| `FM-E019` | Enterprise and equity measures are mixed: a comparison of terminal values declared on one basis contains an exit multiple, a normalised income or a liquidation of the other basis, or the terminal technique of a DCF is an exit multiple, a normalised income or a liquidation of the other basis (P/E and P/BV are equity; EV/EBITDA, EV/EBIT, EV/Sales are enterprise; `dcf.fcff` is enterprise, `dcf.fcfe` is equity); `tv.value_driver` inside `dcf.fcfe` (its terms, NOPAT and RONIC, are firm-level quantities, SPEC-0001 Decision 23); a net financial position given to `dcf.fcfe`, whose FCFE already include the debt | Use one basis throughout: an enterprise multiple with an enterprise comparison or `dcf.fcff`, P/E or P/BV with an equity comparison or `dcf.fcfe`, and declare a normalised income or a liquidation on the basis of the comparison or of the DCF; with `dcf.fcfe`, remove the net financial position and give cash that the FCFE do not reflect in `non_operating_assets`, or value the enterprise with `dcf.fcff` |
| `FM-E020` | Balloon amount out of range: negative, not lower than the principal, or given with a `bullet` schedule | Give a balloon amount of at least 0 and lower than the principal; omit it for `bullet` |
| `FM-E021` | The instalment never repays the loan: it is 0 or below, or it does not exceed the interest of the first period, whatever the sign of the rate | Raise `instalment` above 0 and above periodic rate × principal (for payments in advance, periodic rate × principal / (1 + periodic rate)) |
| `FM-E022` | Principal grace period not lower than the number of instalments, or given with a `bullet` schedule | Give fewer grace periods than instalments; omit them for `bullet`, which is interest only until the last instalment |
| `FM-E023` | With rounding, the outstanding balance would become negative before the last period, or the final-period adjustment would make the last principal portion or instalment negative | Round to more decimal places, or check that the loan is large enough for the rounding chosen |
| `FM-E024` | The cash flows and the discount rates of a DCF are declared in different terms (`nominal` and `real`) and no expected inflation is given | Give expected inflation (`inflation`, or `inflation_per_period` with `terminal_inflation`), so that the discount rates are converted to the terms of the cash flows by the exact Fisher relation; or restate the discount rates or the cash flows in the same terms, so that `cash_flow_terms` equals `discount_rate_terms`. When one of the two was not given, it defaults to `nominal`; the message names it and the value to give |
| `FM-E025` | A count of a DCF does not match its stages: a `given` stage with more or fewer flows than its `periods`, rates or expected inflation per period not one per period of the explicit horizon, a base flow missing for a generated first stage, or a base flow given when the first stage is `given` | Give exactly one flow, rate or inflation per period, as the message states; give `base_flow` only when the first stage is `constant_growth` or `linear_decline` |
| `FM-E026` | The financial statements do not reconcile (`statements.reorganise`): a total given differs from the sum of its components, a net amount from gross less accumulated, total assets from total liabilities and equity, the profit of equity item A.IX from item 21 of the income statement, an "of which" part (B10 c, or B10 a and b without it, within B10 a to c included) or an amount beyond one year exceeds its aggregate, a split of an override does not sum to the year's aggregate, the amounts beyond one year of the parts of a split of shareholder loans exceed a part or differ from the loans', a total of the reclassified form differs from its value derived by Appendix B, total funds invested differ from the sources of a reclassified balance sheet, or of a statutory one once the separate lines taken as zero are counted (item A, and lines D and E presented within C.II or D; the message names them, while with every item given the difference is `FM-E012`), an adjustment (or the interest cost of the employee severance indemnity) exceeds what remains in its source, or the current part of a lease liability exceeds the liability; the message names both sides by the JSON pointers of the items read and gives both amounts and the difference. The comparison is exact, with no tolerance, and uses only items given (a part presented within C.II or D counts as given): a relation with a side missing, or resting on item A or an "of which" part taken as zero, is not checked and reported as `not_checked` | Correct the amount the message names, give the item taken as zero that is not zero, or leave out one side of the relation to have it reported as not checked; for statements filed in euro units, give the rounding reserve within other reserves (A.VI) so that the totals reconcile |
| `FM-E027` | Below the minimum input of a method (ADR 0014 G10.1; `ratios.*`, SPEC-0007 R12): no output has every input it needs (for `ratios.covenants`, no covenant can be tested or decided by its rule), so the result would be made only of nulls; the message names the method and lists the alternative sets of inputs, any one of which is enough, for example ``give at least one of these sets of inputs: [`/closing/cash`] or [`/closing/current_assets`]`` | Give every input of one of the sets listed; the method choices alone compute nothing |
