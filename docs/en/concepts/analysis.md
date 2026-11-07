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
| `analysis.sensitivity` | Planned | How one output moves across a one-way or two-way table of inputs |
| `analysis.scenarios` | Planned | The outputs of named cases, and their deltas against the base case |
| `analysis.tornado` | Planned | Which inputs move one output the most, ranked by swing |

All four belong to the `analysis` theme and follow the rules on this page (SPEC-0008).

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

One request performs at most 10,000 evaluations of the inner method. Where the number of
evaluations is known in advance (a table of 101 × 100 cells), a request above the cap returns
`FM-E016` before the first evaluation. Goal seek needs at most 2 + `max_iterations`
evaluations, and `max_iterations` is capped at 1,000.

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
In sensitivity tables and scenarios a failing cell or scenario records its error and the
others are still returned: a WACC × g table naturally contains cells where growth is not below
the discount rate (`FM-E004`). An internal defect (`FM-E012`) always stops the request.
