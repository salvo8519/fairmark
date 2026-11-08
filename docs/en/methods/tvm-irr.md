# Internal rate of return (`tvm.irr`)

Finds the rate per period at which the net present value of a periodic cash flow series is
zero. The solver is deterministic: it scans a fixed grid of rates, refuses inputs for which
more than one rate solves the equation, and states whether the rate it returns is proven
unique. For flows on calendar dates use [`tvm.xirr`](tvm-xirr.md); to state the finance and
reinvestment rates explicitly use [`tvm.mirr`](tvm-mirr.md).

Method version: 2. Version 2 adds the grid scan, the uniqueness check, the output
`uniqueness_verified` and the error `FM-E011`. Inputs with one sign change and an IRR inside
the grid range give the same result as version 1; inputs whose only rate lies outside the grid
while the flows change sign more than once now return `FM-E011`, and results at negative rates
can differ in the last digits (discount factors are computed from the reciprocal of the base).

## Formula

Finds $r$ such that

$$
\sum_{t=0}^{n} \frac{CF_t}{(1+r)^t} = 0
$$

The first cash flow occurs at $t = 0$, as in [`tvm.npv`](tvm-npv.md).

## Example

```python
from decimal import Decimal

import fairmark as fm

r = fm.tvm.irr([-1000, 300, 400, 500])
print(r.irr)                  # 0.0889633946933499353177656797
print(r.uniqueness_verified)  # True: one sign change in the flows
print(r.solver, r.iterations) # newton 5
assert abs(r.irr - Decimal("0.088963394693")) < Decimal("0.000000000001")

# Flows -100, 230, -132 have two IRRs, 10% and 20%: the engine refuses to choose.
try:
    fm.tvm.irr([-100, 230, -132])
except fm.FairmarkError as err:
    assert err.code == "FM-E011"
    print(err.message)  # no single rate can be isolated: the grid finds roots in {0.10}, {0.20}; ...

# Three sign changes, one rate isolated on the grid (50%); a further pair cannot be excluded.
# Here a pair exists (11% and 12%), so the result is flagged.
flagged = fm.tvm.irr(["-1", "3.73", "-4.5882", "1.8648"])
assert not flagged.uniqueness_verified
print(flagged.conventions[-1])  # uniqueness not verified: the flows change sign 3 times; ...
```

## Solver

1. **Grid scan.** NPV is evaluated on a fixed grid of rates per period (the period of the cash
   flows): −0.99, −0.95, then −0.90 to 1.00 in steps of 0.05, then 1.25 to 10 in steps of 0.25
   (ADR 0008). A grid point where the calculation overflows is skipped. Each sign change
   between consecutive grid points, and each grid point where NPV is exactly zero, is a root
   found. When one root is found, `explain` shows where: the grid point, or the two rates of
   the grid interval with the NPV at each of them.
2. **Uniqueness check.** The roots found are compared with the number $s$ of sign changes in
   the flows. By Descartes' rule of signs the number of rates above −100% is $s$ minus an even
   number. Hence:

    | Roots found on the grid | Sign changes $s$ | Result |
    | --- | --- | --- |
    | more than one | any | `FM-E011`, with the intervals |
    | one | $s - 1$ odd | `FM-E011`: at least one more rate exists |
    | none | $s > 1$ | `FM-E011`: zero or several rates exist |
    | one | $s - 1$ even and positive | rate returned, `uniqueness_verified = false` |
    | one or none | $s = 1$ | rate returned, `uniqueness_verified = true` |

3. **Refinement.** With one interval, Newton's method starts from `guess` (default `0.10`)
   and its result is accepted only inside the interval; otherwise bisection runs inside the
   interval. A root exactly on a grid point is returned as is (solver `grid (exact root)`).
   With no interval and $s = 1$, Newton's method from `guess` is accepted only if it
   converges; otherwise `FM-E005`.
4. **Stopping rule.** Iteration stops when the step on the rate is below $10^{-18}$, at most
   200 iterations per stage.

The remaining limit is the even gap: when one rate is found and $s - 1$ is even and
positive, further rates may exist in pairs (two roots inside one grid interval, or outside
the grid). They cannot be excluded, and the result says so through `uniqueness_verified =
false` and a convention. When the rate is ambiguous, prefer NPV at the cost of capital, or
[`tvm.mirr`](tvm-mirr.md), for the decision.

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `cash_flows` | Cash flows, the first at $t = 0$; at least one positive and one negative | required |
| `guess` | Starting point for Newton's method, decimal fraction per period | `0.10` |

## Outputs

| Field | Meaning |
| --- | --- |
| `irr` | Internal rate of return per period, decimal fraction |
| `uniqueness_verified` | `true` when no other rate can solve the equation; `false` when further rates in pairs cannot be excluded |
| `npv_at_irr` | NPV at the returned rate (about zero) |
| `solver` | `newton`, `bisection` or `grid (exact root)` |
| `iterations` | Iterations of the solver that produced the result |
| `tolerance` | Stopping rule on the step of the rate, $10^{-18}$ |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Value | Default |
| --- | --- | --- |
| Time origin | `cash_flows[0]` at $t = 0$ | fixed |
| Solver grid | rates per period −0.99, −0.95, −0.90 to 1.00 step 0.05, 1.25 to 10 step 0.25 | fixed |
| Solver order | Newton's method inside the root interval, bisection as fallback | fixed |
| Stopping rule | step on the rate below $10^{-18}$, at most 200 iterations | fixed |
| Newton starting point | `guess`, reported as given or default | `0.10` |
| Uniqueness | proven, or not verified with the reason | reported on every result |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | A field is missing or unknown |
| `FM-E002` | `cash_flows` is empty |
| `FM-E005` | No root was bracketed on the grid and Newton's method from `guess` did not converge; the rate may lie outside the decimal range or far from `guess`: give a `guess` near the expected rate |
| `FM-E006` | `cash_flows` has no positive or no negative amount |
| `FM-E008` | Arithmetic overflow while refining the root |
| `FM-E011` | No single rate can be isolated: several rates set NPV to zero, a repeated root is possible, or another rate is known to exist beside the one found; the message lists the intervals and suggests `tvm.npv` at the cost of capital or `tvm.mirr` with explicit rates |

## Accuracy

Each NPV evaluation uses integer powers only; measured against an independent 60-digit
computation, such operations agree to about $10^{-24}$ relative or better. The tolerance of
$10^{-18}$ is a stopping rule on the step of the rate, not an error bound: for a simple root
the returned rate is accurate to about that step, and near a double root (two rates very
close together) accuracy is lower. The golden cases are checked to $10^{-12}$.

## Verified against

- Golden cases in `tests/golden/tvm.irr/`, including a two-rate case derived from the roots
  of the polynomial ($10\%$ and $20\%$).
- Agrees with the spreadsheet `IRR` function (Excel, LibreOffice), `numpy_financial.irr` and
  `pyxirr.irr` on single-rate inputs.
