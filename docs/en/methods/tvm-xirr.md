# Internal rate of return of dated flows (`tvm.xirr`)

Finds the annual rate at which the net present value of dated cash flows is zero. It is the
dated counterpart of [`tvm.irr`](tvm-irr.md): use it for investments whose flows do not fall
on regular periods, such as a fund commitment with irregular calls and distributions. The
solver is deterministic, refuses inputs for which more than one rate solves the equation, and
states whether the rate it returns is proven unique. With the defaults, the result is that of
the spreadsheet `XIRR` function.

Method version: 2. Version 2 refuses a `guess` outside the domain of the rate (the guess per
compounding period at or below −100%) with `FM-E003` before any iteration; version 1 ignored
such a guess. Results for any other guess are unchanged.

## Formula

Finds $r$ such that

$$
\sum_{j} CF_j \cdot DF(r, t_j) = 0,
\qquad t_j = \text{year fraction}(D_1, D_j)
$$

where $D_1$ is the earliest flow date and $DF$ is the discount factor of
[`tvm.xnpv`](tvm-xnpv.md): $(1 + r/m)^{-m t}$, or $e^{-r t}$ with continuous compounding.
With the defaults (Actual/365 (Fixed), annual compounding):

$$
\sum_{j} \frac{CF_j}{(1 + r)^{(D_j - D_1)/365}} = 0
$$

## Example

An outflow of 10,000 on 1 January 2008 and four inflows to 1 April 2009.

```python
from decimal import Decimal

import fairmark as fm

flows = [
    ("2008-01-01", -10000),
    ("2008-03-01", 2750),
    ("2008-10-30", 4250),
    ("2009-02-15", 3250),
    ("2009-04-01", 2750),
]
r = fm.tvm.xirr(flows)
assert abs(r.xirr - Decimal("0.37336253351883151031")) < Decimal("1e-15")
assert abs(r.xirr - Decimal("0.373362535")) < Decimal("0.00000001")  # published value
assert r.uniqueness_verified                # one sign change in date order
print(r.solver, r.iterations, r.tolerance)  # newton 6 1E-18
print(r.xnpv_at_xirr)                       # about zero: check of the result
```

The published value is 0.373362535. Excel stops iterating when the rate is accurate to
0.000001 percent, so its ninth decimal is not significant: the exact value is
0.37336253351883..., which rounds to 0.373362534 at nine decimals.

## Solver

The same deterministic solver as [`tvm.irr`](tvm-irr.md#solver), with two differences:

- **The grid is defined on effective annual rates** (−0.99, −0.95, −0.90 to 1.00 step 0.05,
  1.25 to 10 step 0.25) and converted to the chosen compounding before the scan. The range of
  economic rates scanned, and therefore the verdict on uniqueness, does not depend on how the
  rate is quoted. When one root is found, `explain` shows where: the grid point, or the two
  rates of the grid interval with the XNPV at each of them.
- **Sign changes are counted on the flows in date order**; flows on the same date are netted.

The uniqueness rule is the one of `tvm.irr`: more than one root found, an odd gap between the
sign changes $s$ and the roots found, or no root found with $s > 1$ give `FM-E011`. One root
found with an even, positive gap is returned with `uniqueness_verified = false`, because
further rates in pairs cannot be excluded. With $s = 1$ the rate is proven unique.

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `cash_flows` | Dated flows `{date, amount}`, in any order; at least one positive and one negative amount | required |
| `day_count` | `act_365_fixed`, `act_360`, `act_act_isda`, `thirty_360` or `thirty_e_360` | `act_365_fixed` |
| `compounding` | Compounding of the returned rate: `annual`, `semi_annual`, `four_monthly` (3 a year), `quarterly`, `bimonthly` (every two months, 6 a year), `monthly` or `continuous` | `annual` |
| `guess` | Starting point for Newton's method, annual decimal fraction in the compounding of the result, greater than $-m$, with $m$ the compounding periods a year (−1 annual, −2 semi-annual, −3 four-monthly, −4 quarterly, −6 bimonthly, −12 monthly; no bound for continuous) | `0.10` |

## Outputs

| Field | Meaning |
| --- | --- |
| `xirr` | Annual rate at which XNPV is zero, decimal fraction, with the chosen compounding |
| `uniqueness_verified` | `true` when no other rate can solve the equation; `false` when further rates in pairs cannot be excluded |
| `xnpv_at_xirr` | XNPV at the returned rate (about zero) |
| `solver` | `newton`, `bisection` or `grid (exact root)` |
| `iterations` | Iterations of the solver that produced the result |
| `tolerance` | Stopping rule on the step of the rate, $10^{-18}$ |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Day count | five day counts, see [`dates.year_fraction`](dates-year-fraction.md) | Actual/365 (Fixed), the ECMA-376 spreadsheet default |
| Compounding | annual, semi-annual, four-monthly, quarterly, bimonthly, monthly, continuous | annual, the ECMA-376 spreadsheet default |
| Time origin | earliest flow date | fixed |
| Solver grid | effective annual rates −0.99 to 10, converted to the compounding | fixed |
| Stopping rule | step on the rate below $10^{-18}$, at most 200 iterations | fixed |
| Newton starting point | `guess` | `0.10` |

Each convention is reported with whether it was given or is the default, and the result
states whether uniqueness is proven or not verified, with the reason.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | A field is missing or unknown, or a convention is not one of the listed options |
| `FM-E002` | `cash_flows` is empty |
| `FM-E003` | `guess` is $-m$ or lower, with $m$ the compounding periods a year (with annual compounding, `guess` ≤ −1): Newton's method cannot start there; the message names `/guess` and suggests a guess near the expected rate or leaving out `guess` to use the default 0.10. Checked before any iteration |
| `FM-E005` | No root was bracketed on the grid and Newton's method from `guess` did not converge; the rate may lie outside the decimal range or far from `guess`: give a `guess` near the expected rate |
| `FM-E006` | `cash_flows` has no positive or no negative amount, as given or after flows on the same date are netted (the message says which) |
| `FM-E008` | Arithmetic overflow while refining the root |
| `FM-E009` | A date is malformed, does not exist or is outside 1900-01-01 to 2199-12-31 |
| `FM-E011` | No single rate can be isolated: several rates set XNPV to zero, a repeated root is possible, or another rate is known to exist beside the one found; the message lists the intervals and suggests `tvm.npv` at the cost of capital or `tvm.mirr` with explicit rates |

## Accuracy

Year fractions are exact ratios. Discount factors with a fractional exponent use the decimal
power function, and continuous compounding the decimal exponential. Measured against an
independent 60-digit computation, these agree to about $10^{-18}$ relative or better on
typical inputs; integer powers agree to about $10^{-24}$ relative or better. The tolerance of
$10^{-18}$ is a stopping rule on the step of the rate, not an error bound, and accuracy is
lower near a double root. The full-precision golden case is checked to $10^{-19}$.

## Verified against

- Published example: ECMA-376 / Excel `XIRR` example, 0.373362535.
- Agrees with the `XIRR` function of Excel and LibreOffice and with `pyxirr.xirr`.
- Day counts: definitions of the 2006 ISDA Definitions, section 4.16.
