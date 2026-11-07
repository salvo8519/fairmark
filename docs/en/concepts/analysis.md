# Analysis methods

A valuation is a point estimate built on uncertain assumptions. Reviewers ask the same
questions every time: which discount rate gives this price, how the value moves across a range
of assumptions, what the value is in a downside case. Analysis methods answer them inside the
engine, with every evaluation reproducible and every convention reported.

An analysis method does not value anything itself. It evaluates another registered method, the
inner method, repeatedly, changing one or more of its decimal inputs, and reports the outputs.

| Method | Status | Question it answers |
| --- | --- | --- |
| [`analysis.goal_seek`](../methods/analysis-goal-seek.md) | Available | Which value of one input makes one output equal a target |
| [`analysis.sensitivity`](../methods/analysis-sensitivity.md) | Available | How one output moves across a one-way or two-way table of input values |
| [`analysis.scenarios`](../methods/analysis-scenarios.md) | Available | What the outputs are under named sets of input changes, with deltas against the base case |
| [`analysis.tornado`](../methods/analysis-tornado.md) | Available | Which inputs move one output the most, each varied alone between a low and a high value |

The four methods are specified in SPEC-0008, and every method of the `analysis` theme follows
the rules on this page. Sensitivity tables, scenarios and tornado first evaluate `base_input`
unchanged and report its outputs as the base case.

## Composition through the registry

The inner method is chosen by the caller at run time, by its id: `"method": "dcf.fcff"`. The
analysis passes `base_input` to the registry inside `fairmark-core`, exactly as an adapter
would pass it to `run`. The inner method parses, validates and computes as it always does, so
its own schema and its own errors apply (ADR 0010). Consequences:

- Any method works as an inner method, including methods added later, with no change to the
  analysis.
- The schema of an analysis method describes `base_input` only as an object; the inner method
  validates it at run time. Analysis schemas do not change when an inner method does.
- Every result records the inner method and the version this engine ran, in `inner_method`.
  Reproducing a result needs that version and the engine version in `meta`.
- Composition is one level deep: an analysis method cannot evaluate another analysis method
  (`FM-E017`).

## Pointers

Variables and outputs are named by JSON Pointers (RFC 6901), the standard notation for one
location in a JSON document:

| Pointer | Refers to |
| --- | --- |
| `/discount_rate` | the member `discount_rate` |
| `/free_cash_flows/0` | the first element of the list `free_cash_flows` |
| `/a~1b` | the member `a/b` (`~1` stands for `/`, `~0` for `~`) |

A variable pointer resolves in `base_input`; an output pointer resolves in the `value` of the
inner result. Both must start with `/` and lead to an existing decimal, given as a JSON number
or a string that parses as a decimal. A pointer that does not resolve, that uses the `-` token
(past the end of a list), or that leads to text, a list or an object returns `FM-E015` naming
the pointer. Variable pointers are checked before any evaluation, output pointers on the first.

Only decimal inputs can vary. Integers such as a number of periods, choices such as `timing`
and whole lists are out of scope.

## Variables are written as decimal strings

When the analysis sets a variable, it writes the decimal as a JSON string at the pointer and
leaves every other member of `base_input` unchanged. Strings keep the value exact: no binary
floating point is involved. A base input that gives a value as the number `0.1` or as the
string `"0.1"` produces an identical `value`; `meta.input_hash` differs, because the hash
covers the request as written.

## Bounded cost

One request performs at most 10,000 evaluations of the inner method, the base case included.
The number of evaluations is known in advance, so a request above the cap returns `FM-E016`
before the first evaluation:

| Method | Evaluations |
| --- | --- |
| Goal seek | at most 2 + `max_iterations`, and `max_iterations` is capped at 1,000 |
| Sensitivity table | rows × columns + 1, so at most 9,999 cells (a 100 × 100 table is refused) |
| Scenarios | scenarios + 1 |
| Tornado | 2 × variables + 1 |

Failed cells and failed scenarios count against the limit like any other evaluation.

Each evaluation parses, validates and computes the whole inner input, so the cost also grows
with the size of `base_input`. A second bound applies: evaluations × values in `base_input`
(every number, string, list and object counts one) at most 1,000,000. A 21 × 21 table on a ten-year DCF
uses about 6,000 units; a 60 × 60 table on a DCF with 300 yearly flows is refused with
`FM-E016`, again before the first evaluation.

## Determinism

Evaluations run one after another in a fixed, documented order: the bracket ends then the
iterations in goal seek; rows then columns in a table; scenarios and tornado variables in
declaration order, low before high. The same input and versions give the same output bit for
bit, including the order of `explain.steps`.

`meta.input_hash` is computed with the keys of `base_input` sorted recursively, so two requests
that differ only in key order have the same hash. See [the determinism contract](determinism.md).

## Errors of the inner method

An error raised by the inner method keeps its own code. In goal seek and in tornado any such
error stops the request, because the search or the ranking is not defined without that value.

In sensitivity tables and scenarios the errors are per cell: a failing cell or scenario holds
`{"error": {"code", "message"}}` in place of its value, the others are still returned, and the
result counts them in `failed_cells` or `failed_scenarios`. A WACC × g table naturally contains
cells where growth is not below the discount rate (`FM-E004`), and a spreadsheet data table
shows an error in those cells too. Two errors still stop the request: an error on the base case,
which every other cell or scenario is read against, and an internal defect (`FM-E012`).
