# Goal seek (`analysis.goal_seek`)

Finds the value of one decimal input of any registered method at which one of its decimal
outputs equals a target. Typical questions: the discount rate at which the DCF enterprise value
equals an offer price (the implied return of the offer), or the rate at which an NPV is zero.
The search runs inside an interval given by the caller, every evaluation of the inner method is
reported, and the result carries the inner input at the solution so that anyone can rerun it.

Goal seek is an analysis method: it evaluates another method through the registry (see
[Analysis methods](../concepts/analysis.md)). It cannot evaluate another analysis method.

## Formula

Given the inner method $M$, the variable $x$ (one decimal input of $M$), the output $y = M(x)$
and the target $T$, goal seek finds $x^*$ in the bracket $[x_{low}, x_{high}]$ such that

$$
\lvert f(x^*) \rvert \le \text{tolerance},
\qquad
f(x) = y(x) - T
$$

The tolerance is absolute and measured on the output, in the output's units (currency for an
enterprise value, a decimal fraction for a rate).

## Algorithm

Brent's method in its textbook formulation, the one implemented by `scipy.optimize.brentq`:

1. Evaluate both ends of the bracket, low first. If an end already meets the tolerance it is
   returned with 0 iterations. If $f(x_{low})$ and $f(x_{high})$ have the same strict sign, the
   target is not bracketed (`FM-E013`). There is no automatic search for a wider bracket.
2. Each iteration takes one new point by inverse quadratic interpolation through the last three
   points, by the secant through the last two, or by bisection of the current bracket. An
   interpolated step is accepted only when it stays well inside the bracket and shrinks fast
   enough; otherwise the iteration bisects. An interpolation that would overflow also falls
   back to bisection.
3. The step on the variable is never smaller than $10^{-24} \times \max(1, \lvert x \rvert)$,
   which keeps each new point distinct at the 28 significant digits of the decimal type. Every
   new point lies strictly inside the current bracket, which always lies inside yours.
4. The search stops at the first point with $\lvert f(x) \rvert \le$ `tolerance`. If
   `max_iterations` points after the two bracket ends do not meet it, the result is `FM-E014`.

The output need not be monotonic, but it must be continuous in the variable over the bracket:
when the output jumps across the target, the bracket shrinks to the minimum step without
meeting the tolerance and the result is `FM-E014`, naming the collapsed interval, never a false
solution.

Two limits to keep in mind:

- **Several solutions.** When the bracket contains more than one solution (non-conventional
  cash flows can have several IRRs), one of them is returned, determined by the iteration path,
  and the result is the same on every run. Narrow the bracket to select another.
- **Flat outputs.** The tolerance applies to the output, not to the variable. Where the output
  barely moves with the variable, a point that meets the tolerance can be far from the exact
  solution; tighten the tolerance in that case.

## Example

The DCF of [`dcf.fcff`](dcf-fcff.md) values free cash flows of 100, 110 and 120 with 2%
terminal growth. At which discount rate is the enterprise value equal to an offer of 1300?

```python
from decimal import Decimal

import fairmark as fm

base = {
    "stages": [{"periods": 3, "projection": "given", "flows": [100, 110, 120]}],
    "cash_flow_terms": "nominal",
    "discount_rate_terms": "nominal",
    "wacc": "0.10",
    "timing": "end_of_period",
    "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
    "net_financial_position": {"gross_debt": 200, "cash": 0},
}
r = fm.analysis.goal_seek(
    "dcf.fcff",
    base,
    variable="/wacc",
    output="/enterprise_value",
    target="1300",
    bracket=("0.05", "0.5"),
)
print(r.solution)    # 0.1073730997872031000512537941
print(r.achieved)    # 1300.0000000000600441199177741
print(f"{r.residual:f}")  # 0.0000000000600441199177741, within the default tolerance 0.000000001
print(r.iterations)  # 9, after the two bracket ends
assert abs(r.solution - Decimal("0.107373099787207080")) < Decimal("0.000000000001")

# The same request through the generic entry point, as an agent or the CLI sends it.
same = fm.run("analysis.goal_seek", {
    "method": "dcf.fcff",
    "base_input": base,
    "variable": "/wacc",
    "output": "/enterprise_value",
    "target": "1300",
    "bracket": ["0.05", "0.5"],
})
assert same.solution == r.solution

# Rerun the inner method at the solution to get its full explain.
inner = fm.run(r.inner_method["id"], r.solved_input)
assert inner.value == r.inner_value
print(inner.steps[0].label)  # first step of the DCF audit trail
```

The implied discount rate of the offer is about 10.74%. The steps of the result list the 11
evaluations in order (two bracket ends, then secant, bisection and inverse quadratic
interpolation steps) with the rate, the enterprise value and the residual of each.

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `method` | Id of the method to evaluate, for example `dcf.fcff`; analysis methods are not accepted | required |
| `base_input` | Input of that method, complete, exactly as the method accepts it, including the variable at any starting value | required |
| `variable` | RFC 6901 JSON Pointer to the decimal input to solve for, in `base_input`, for example `/discount_rate` or `/cash_flows/0` | required |
| `output` | RFC 6901 JSON Pointer to the decimal output, in the `value` of the inner result, for example `/enterprise_value` | required |
| `target` | Value the output must reach, in the output's units | required |
| `bracket` | `[low, high]` with low < high: the interval searched for the variable, in the variable's units | required |
| `tolerance` | Absolute tolerance on the output, in the output's units, greater than 0 | `0.000000001` |
| `max_iterations` | Largest number of evaluations after the two bracket ends, from 1 to 1,000 | `100` |

Choose the tolerance in the output's units. For an enterprise value in euro, the default of
$10^{-9}$ euro is far below any meaningful amount; for an output that is a rate, it is
$10^{-7}$ percentage points. A tolerance of $\varepsilon$ on the output fixes the variable to
about $\varepsilon / \lvert dy/dx \rvert$.

## Outputs

| Field | Meaning |
| --- | --- |
| `inner_method` | `id` and `version` of the method evaluated, at the version this engine ran |
| `solution` | Value of the variable at which the output meets the target |
| `achieved` | Output of the inner method at the solution |
| `residual` | `achieved − target`; its absolute value is at most the tolerance |
| `iterations` | Evaluations after the two bracket ends (0 when an end meets the target) |
| `tolerance` | Tolerance applied, in the output's units |
| `solved_input` | Input of the inner method at the solution, with the variable written as a decimal string |
| `inner_value` | `value` of the inner result at the solution |

`solved_input` and `inner_value` make the solution reproducible: running the inner method on
`solved_input` returns `inner_value` and the inner method's full explain, which goal seek does
not repeat. Their schemas are those of the inner method: in the output schema of goal seek
(`fairmark schema analysis.goal_seek --output`) they carry `x-fairmark-input-of` and
`x-fairmark-output-of`, set to `/inner_method/id`, and the Python SDK decodes them with the
inner method's input and output schemas, so their decimals are `Decimal` and their text stays
text.

The steps list one entry per evaluation, labelled `Evaluation k (bracket end)`, `(bisection)`,
`(secant)` or `(inverse quadratic interpolation)`, with the variable, the output and the
residual substituted, then a final `Solution` step.

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Value | Default |
| --- | --- | --- |
| Inner method | id and version, evaluated through the registry | required |
| Inner method status | when the inner method is preview, "Evaluated method <id> is preview"; the status is also in `meta.evaluated_method_status` | reported |
| Root finding | Brent's method: inverse quadratic interpolation, secant, bisection | fixed |
| Bracket | given by the caller; no automatic search | required |
| Both ends meet the tolerance | low is returned | fixed |
| Several solutions in the bracket | one is returned, determined by the iteration path | fixed |
| Stopping rule | $\lvert \text{output} - \text{target} \rvert \le$ `tolerance`, absolute, in output units | `0.000000001` |
| Iteration cap | `max_iterations` after the two bracket ends | `100` |
| Minimum step on the variable | $10^{-24} \times \max(1, \lvert x \rvert)$ at the current point | fixed |
| Variable encoding | written into the inner input as a decimal string | fixed |
| Evaluation order | low end, high end, then one evaluation per iteration | fixed |

The last convention reported is the number of evaluations of the inner method.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | low ≥ high, tolerance ≤ 0, `max_iterations` outside 1 to 1,000, or a field is missing or unknown |
| `FM-E007` | `method` names no registered method |
| `FM-E013` | The output minus the target has the same sign at both ends of the bracket and neither end meets the tolerance; the message reports both values |
| `FM-E014` | The tolerance was not met within `max_iterations`, or the output jumps across the target; the message reports the closest evaluation and the interval that still brackets the target |
| `FM-E015` | `variable` does not resolve to a decimal in `base_input`, or `output` does not resolve to a decimal in the inner result; the message names the pointer |
| `FM-E016` | The request would exceed 10,000 inner evaluations; goal seek needs at most 1,002, so this code concerns the other analysis methods |
| `FM-E017` | `method` names an analysis method |

`variable` is checked before any evaluation, `output` on the first one. `base_input` is also
checked before any evaluation, with the variable at the low end of the bracket, by the
validation alone of the inner method (every check of its input, nothing computed); its input
errors are reported under `/base_input` with the other errors of the request. An error of the inner
method at any evaluation is returned unchanged, with its own code: for example `FM-E004` when a
discount rate in the bracket is at or below the terminal growth of `dcf.fcff`. A root search
over a partly undefined function is not defined, so choose a bracket on which the inner method
is valid.

## Accuracy

The stopping rule bounds the residual on the output, not the error on the variable. Each
evaluation is as accurate as the inner method (see its page). In the example, the solution
differs from an independent 50-digit computation by about $4 \times 10^{-15}$; the golden
cases are checked to $10^{-12}$ (DCF) and $10^{-15}$ (IRR through NPV, with tolerance
$10^{-18}$).

## Verified against

- Golden cases in `tests/golden/analysis.goal_seek/`, whose expected solutions come from
  `scripts/reference_analysis.py`: bisection in Python's `decimal` at 50 digits to $10^{-30}$
  on the variable, with `dcf.fcff` and `tvm.npv` restated from their documented formulas. It
  shares no code with the engine.
- The rate at which the NPV of −1000, 300, 400, 500 is zero agrees with
  [`tvm.irr`](tvm-irr.md) and with `pyxirr.irr`.
- The same script holds Brent's method written from its textbook formulation (Numerical
  Recipes, `zbrent`). A test requires the engine to reproduce its iteration count and solution,
  including a case with three IRRs in the bracket, so any change to the solver path is caught.
