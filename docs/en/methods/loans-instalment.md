# Loan instalment (`loans.instalment`)

Returns the constant instalment of a loan from its principal, its rate, the number of
instalments and the payment timing, with an optional balloon amount repaid with the last
instalment and an optional rounding of the instalment. Use it to put new bank debt into a
business plan or to check the instalment quoted by a lender.

## Formula

With the periodic rate $i$, the number of instalments $n$, the principal $P$ and the balloon
amount $B$ (paid at the end of period $n$), the instalment $A$ makes the present value of the
instalments and of the balloon amount equal to the principal:

$$
A_{\text{arrears}} = \frac{\left(P - B\,(1 + i)^{-n}\right) i}{1 - (1 + i)^{-n}}
\qquad
A_{\text{advance}} = \frac{A_{\text{arrears}}}{1 + i}
\qquad
A = \frac{P - B}{n} \text{ when } i = 0
$$

The engine computes $A$ as $(P - B\,(1 + i)^{-n})$ divided by the annuity factor of
[`tvm.annuity`](tvm-annuity.md), so both methods share one implementation.

### The periodic rate

The rate is given in exactly one of two forms:

- **Form (a)**: an annual borrowing rate `rate` with its `compounding` and the
  `payment_frequency` of the instalments ($p$ a year). The periodic rate is the equivalent rate
  with compounding equal to the payment frequency (as [`rates.convert`](rates-convert.md)
  computes it), divided by $p$. When the compounding equals the payment frequency this is
  simply `rate` / $p$: 6% compounded monthly with monthly instalments gives 0.5% a month.
- **Form (b)**: a `periodic_rate`, the rate per payment period, as
  [`tvm.annuity`](tvm-annuity.md) takes it.

There is no default compounding: a borrowing rate means nothing without it.

## Example

The ECMA-376 worked example (10,000 at 8% a year, monthly compounding and monthly
instalments, 10 instalments), and a ten-year loan with a balloon of 50,000 rounded to the cent:

```python
from decimal import Decimal

import fairmark as fm

monthly = {"rate": "0.08", "compounding": "monthly", "payment_frequency": "monthly"}
arrears = fm.loans.instalment(10000, 10, timing="ordinary", **monthly)
advance = fm.loans.instalment(10000, 10, timing="due", **monthly)
assert round(arrears.instalment, 2) == Decimal("1037.03")
assert round(advance.instalment, 2) == Decimal("1030.16")

balloon = fm.loans.instalment(
    250000, 120, rate="0.054", compounding="monthly", payment_frequency="monthly",
    timing="ordinary", balloon_amount=50000, decimal_places=2, rounding_mode="half_up",
)
assert balloon.instalment == Decimal("2385.63")
assert balloon.total_paid == Decimal("336275.60")  # 120 x 2385.63 + 50000
print(balloon.conventions[-2])  # rounding: 2 decimal places, half_up, applied to the instalment

# The same loan with the periodic rate given directly (form b).
same = fm.loans.instalment(20000, 36, periodic_rate="0.005", timing="ordinary")
again = fm.loans.instalment(20000, 36, rate="0.06", compounding="monthly",
                            payment_frequency="monthly", timing="ordinary")
assert same.instalment == again.instalment
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `principal` | Amount borrowed, currency units, greater than 0 | required |
| `rate` | Annual borrowing rate, decimal fraction (form a) | one rate form required |
| `compounding` | Compounding of `rate`: `annual`, `semi_annual`, `four_monthly`, `quarterly`, `bimonthly`, `monthly`, `continuous` (form a) | required with `rate` |
| `payment_frequency` | Instalments per year: `annual` (1), `semi_annual` (2), `four_monthly` (3, every four months), `quarterly` (4), `bimonthly` (6, every two months, not twice a month), `monthly` (12) (form a) | required with `rate` |
| `periodic_rate` | Rate per payment period, decimal fraction, greater than −1 (form b) | one rate form required |
| `number_of_instalments` | From 1 to 10000 | required |
| `timing` | `ordinary` (in arrears) or `due` (in advance) | required |
| `balloon_amount` | Repaid at the end of the last period on top of the last instalment, $0 \le B < P$ | `0` |
| `rounding` | `decimal_places` (0 to 6) and `mode` (`half_up` or `half_even`), both required | none |

## Outputs

| Field | Meaning |
| --- | --- |
| `instalment` | Constant instalment (rounded when `rounding` is given) |
| `periodic_rate` | Rate per payment period used |
| `total_paid` | $n \times A + B$, with the instalment as returned |
| `total_interest` | Total paid less the principal |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Rate form | (a) annual rate with compounding and payment frequency, or (b) periodic rate | exactly one, required |
| Periodic rate (form a) | equivalent rate with compounding equal to the payment frequency, divided by the instalments per year | fixed |
| Timing | `ordinary`: instalment $k$ at the end of period $k$; `due`: at the beginning | none: must be chosen |
| Balloon amount | paid with the last instalment, at the end of period $n$ for both timings | 0, reported as default |
| Rounding | none, or 0 to 6 decimal places with `half_up` (halves away from zero) or `half_even` (halves to the even digit) | none |
| Rate zero | limit form $(P - B) / n$, shown as a separate step | applied when $i = 0$ |

With rounding, the last instalment of a real schedule differs from the others by the
accumulated rounding: [`loans.amortisation_schedule`](loans-amortisation-schedule.md) computes
that final-period adjustment. `total_paid` here uses the rounded instalment for every period.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Principal 0 or negative, `number_of_instalments` 0 or above 10000, both rate forms, neither, form (a) incomplete, `decimal_places` above 6, rounding without a mode, or a field missing or unknown |
| `FM-E003` | `periodic_rate` is −100% or lower, or `rate` is at or below −100% per compounding period ($r/m$); the message names the field, `/periodic_rate` or `/rate` |
| `FM-E020` | The balloon amount is negative or not lower than the principal |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

The instalment uses integer powers, multiplication and division, and holds to the last of the
engine's 28 significant digits. When the compounding differs from the payment frequency, the
conversion uses the series functions of the decimal library (relative error near $10^{-27}$).
The golden cases are checked to $10^{-20}$ against an independent 50-digit reference.

## Verified against

- Golden cases in `tests/golden/loans.instalment/`: the ECMA-376 worked example of the loan
  instalment (in arrears and in advance) and an independent 50-digit reference
  (`scripts/reference_loans.py`), including payments every two months in form (a), equal to the
  equivalent periodic rate in form (b).
- The loan payment function of `numpy_financial` on random loans, both timings, with the
  balloon amount as the future value.
