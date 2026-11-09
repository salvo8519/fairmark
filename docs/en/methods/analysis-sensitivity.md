# Sensitivity table (`analysis.sensitivity`)

Reports one decimal output of any registered method across a range of values of one input
(a one-way table) or of two inputs (a two-way table): the data table of spreadsheet models.
Typical question: how the DCF enterprise value moves across a grid of discount rates and
terminal growth rates. A combination the inner method refuses, such as growth not below the
discount rate, is recorded in its cell and the rest of the table is still returned.

Sensitivity is an analysis method: it evaluates another method through the registry (see
[Analysis methods](../concepts/analysis.md)). It cannot evaluate another analysis method.

## Formula

Given the inner method $M$, the row variable $x_r$ with values $r_1, \dots, r_m$, the optional
column variable $x_c$ with values $c_1, \dots, c_n$ and the output $y$, each cell is

$$
\text{cell}_{ij} = y\big(M(\text{base input with } x_r = r_i \text{ and } x_c = c_j)\big),
\qquad i = 1, \dots, m, \quad j = 1, \dots, n
$$

Every other input keeps its base value. A one-way table has no column variable and one column
($n = 1$). The base case, $y(M(\text{base input}))$, is evaluated first and reported as `base`.

## Example

The five-year DCF of [`dcf.fcff`](dcf-fcff.md): free cash flows of 100, 110, 120, 130 and 140
currency units, a discount rate of 9%, terminal growth of 2% and net debt of 300. The table
shows the enterprise value for discount rates of 8%, 9% and 10% (rows) and terminal growth of
1%, 2% and 8% (columns).

```python
from decimal import Decimal

import fairmark as fm

base = {
    "stages": [{"periods": 5, "projection": "given", "flows": [100, 110, 120, 130, 140]}],
    "cash_flow_terms": "nominal",
    "discount_rate_terms": "nominal",
    "wacc": "0.09",
    "timing": "end_of_period",
    "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
    "net_financial_position": {"gross_debt": 300, "cash": 0},
}
table = fm.analysis.sensitivity(
    "dcf.fcff",
    base,
    output="/enterprise_value",
    rows=("/wacc", ["0.08", "0.09", "0.10"]),
    columns=("/terminal/tv.gordon/growth", ["0.01", "0.02", "0.08"]),
)
print(table.base)                    # 1785.9356357015730744749501235
print(table.cells[0][0]["value"])    # 1847.7733182046548911356105383, rate 8%, growth 1%
print(table.cells[0][2]["error"])    # growth 8% is not below the rate 8%: FM-E004
print(table.failed_cells)            # 1

# Expected values from an independent 50-digit computation (scripts/reference_analysis.py).
expected = {
    (0, 0): "1847.7733182046548911356105381406",
    (1, 1): "1785.9356357015730744749501235001",
    (2, 2): "5141.8618946793251827060993101564",
}
for (i, j), value in expected.items():
    assert abs(table.cells[i][j]["value"] - Decimal(value)) < Decimal("0.000000001")
assert table.cells[0][2]["error"]["code"] == "FM-E004"
assert table.cells[1][1]["value"] == table.base

# A one-way table equals the matching column of the two-way table.
one_way = fm.analysis.sensitivity(
    "dcf.fcff",
    base,
    output="/enterprise_value",
    rows=("/wacc", ["0.08", "0.09", "0.10"]),
)
assert [row[0] for row in one_way.cells] == [row[1] for row in table.cells]

# The same request through the generic entry point, as an agent or the CLI sends it.
same = fm.run("analysis.sensitivity", {
    "method": "dcf.fcff",
    "base_input": base,
    "output": "/enterprise_value",
    "rows": {"variable": "/wacc", "values": ["0.08", "0.09", "0.10"]},
    "columns": {"variable": "/terminal/tv.gordon/growth", "values": ["0.01", "0.02", "0.08"]},
})
assert same.cells == table.cells
```

The cell at 8% and 8% holds `FM-E004` because a Gordon terminal value needs growth below the
discount rate; the other eight cells are values. The steps of the result list the base case,
then one step per successful cell, labelled `Cell (i, j)` (1-based) with both variables
substituted.

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `method` | Id of the method to evaluate, for example `dcf.fcff`; analysis methods are not accepted | required |
| `base_input` | Input of that method, complete, exactly as the method accepts it: the base case | required |
| `output` | RFC 6901 JSON Pointer to the decimal output reported in each cell, in the `value` of the inner result, for example `/enterprise_value` | required |
| `rows.variable` | RFC 6901 JSON Pointer to the decimal input that changes along the rows, in `base_input`, for example `/discount_rate` | required |
| `rows.values` | Values of the row variable, in its units, in table order; at least one | required |
| `columns.variable` | Pointer to a second decimal input, different from the row variable | absent: one-way table |
| `columns.values` | Values of the column variable, in its units, in table order; at least one | required with `columns` |

Values are in the units of the variable: a decimal fraction for a rate (`"0.08"` is 8%),
currency units for an amount. In Python, `rows` and `columns` are `(pointer, values)` pairs.

## Outputs

| Field | Meaning |
| --- | --- |
| `inner_method` | `id` and `version` of the method evaluated, at the version this engine ran |
| `output` | The output pointer reported in each cell |
| `base` | The output for `base_input` unchanged, in the output's units |
| `rows` | The row axis, as given |
| `columns` | The column axis, as given; absent for a one-way table |
| `cells` | `cells[i][j]`: `{"value": ...}` in the output's units, or `{"error": {"code", "message"}}` when the inner method refused that combination; a one-way table has one column |
| `failed_cells` | Number of cells holding an error |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Value | Default |
| --- | --- | --- |
| Inner method | id and version, evaluated through the registry | required |
| Inner method status | when the inner method is preview, "Evaluated method <id> is preview"; the status is also in `meta.evaluated_method_status` | reported |
| Output | the pointer reported in each cell | required |
| Evaluation order | base case, then the cells row by row (row-major), columns in the order given | fixed |
| Failed cells | a cell the inner method refuses holds its error code and message and the table goes on; `FM-E012` (a defect in Fairmark) aborts the request | fixed |
| Variable encoding | written into the inner input as decimal strings | fixed |

The last convention reported is the number of evaluations of the inner method and the number of
failed cells. A table needs rows × columns + 1 evaluations (the base case included); the limit
of 10,000 allows at most 9,999 cells.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | The row and column variables are the same pointer, an axis has no values, or a field is missing or unknown |
| `FM-E007` | `method` names no registered method |
| `FM-E015` | A row or column variable does not resolve to a decimal in `base_input`, or `output` does not resolve to a decimal in the result of the base case; the message names the pointer |
| `FM-E016` | rows × columns + 1 exceeds 10,000 evaluations or the evaluations × values in `base_input` exceed 1,000,000; returned before the first evaluation |
| `FM-E017` | `method` names an analysis method |

Variable pointers, the syntax of `output` and `base_input` are checked before any evaluation,
`base_input` by the validation alone of the inner method (every check of its input, nothing
computed, its input errors reported under `/base_input`); `output` is
resolved on the base case. An error of the inner method on the base case is returned unchanged,
with its own code: without a base case the table has no reference. An error of the inner method
in a cell (for example `FM-E004` in `dcf.fcff`, or `FM-E001` when a value is outside the range
the inner method accepts) is recorded in that cell, except `FM-E012`, which aborts the request.

## Accuracy

Each cell is as accurate as the inner method (see its page); the table adds no arithmetic of
its own. In the example the cells differ from an independent 50-digit computation by less than
$10^{-24}$ currency units; the golden cases are checked to $10^{-9}$ currency units.

## Verified against

- Golden cases in `tests/golden/analysis.sensitivity/` (a 5 × 5 WACC by growth table with two
  refused cells, and a one-way table), whose expected values come from
  `scripts/reference_analysis.py`: `dcf.fcff` restated from its documented formula in Python's
  `decimal` at 50 digits. It shares no code with the engine.
- An independent recomputation of each cell with numpy-financial (`npf.npv` plus the Gordon
  terminal value) agrees to a relative $10^{-9}$ (`python/tests/test_cross_check.py`).
- A property test requires every cell to equal a direct run of the inner method on the
  overridden input, or its error code, and a one-way table to equal the matching column of a
  two-way table.
