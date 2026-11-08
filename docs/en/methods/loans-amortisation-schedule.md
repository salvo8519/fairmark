# Amortisation schedule (`loans.amortisation_schedule`)

Returns the amortisation schedule of a loan: for every period the instalment, the interest
portion, the principal portion and the outstanding balance, with the totals and the
final-period adjustment that brings the balance exactly to zero. Three amortisation types are
available, with an optional initial interest-only period and an optional balloon amount. Use
it to feed the interest of each year and the outstanding debt into a business plan and the net
financial position, or to reconcile a schedule with the bank's.

## Amortisation types

The names are those of the European credit reporting regulation (Regulation (EU) 2016/867,
Annex IV, "amortisation type"):

| Type | What stays constant | Italian practice |
| --- | --- | --- |
| `french` | the instalment: the total of principal and interest is the same in each instalment | ammortamento alla francese (a rata costante) |
| `fixed_amortisation_schedule` | the principal portion: the principal repaid in each instalment is the same | ammortamento all'italiana (a quota capitale costante) |
| `bullet` | nothing but the interest: the full principal is repaid with the last instalment | bullet |

## Formula

With the periodic rate $i$, $n$ instalments (the full term), $g$ interest-only periods at the
start (principal grace period) and a balloon amount $B$ repaid with the last instalment, each
period $k$ starts from the opening balance $D_{k-1}$ ($D_0 = P$):

$$
I_k = i\,D_{k-1}
\qquad
C_k =
\begin{cases}
0 & k \le g \text{ (grace period) or bullet before the last period} \\
A - I_k & \text{french} \\
(P - B) / (n - g) & \text{fixed amortisation schedule}
\end{cases}
\qquad
D_k = D_{k-1} - C_k
$$

The instalment of period $k$ is $I_k + C_k$. For `french`, $A$ is the constant instalment of
[`loans.instalment`](loans-instalment.md) over the $n - g$ periods after the grace period, with
the balloon amount (the same implementation):

$$
A = \frac{\left(P - B\,(1 + i)^{-(n-g)}\right) i}{1 - (1 + i)^{-(n-g)}}
$$

**Last period.** The principal portion of period $n$ is the opening balance $D_{n-1}$, balloon
amount included, so the final outstanding balance is exactly zero. The difference between the
last instalment, less the balloon amount, and the regular instalment of that period is
reported as `final_adjustment`. Without rounding it is zero up to the last digits of the
arithmetic; with rounding it absorbs the accumulated rounding.

**Rounding.** When chosen, rounding applies to the amount the type keeps constant, so the type
keeps its meaning: the instalment of a French schedule, the principal portion of a fixed
amortisation schedule. Interest portions are never rounded. A bullet schedule has no constant
amount to round other than its interest, so rounding does not change it.

Instalment $k$ is paid at the end of period $k$ (time $k$): this method version supports
payments in arrears only.

## Example

The worked example of a business loan in Watts, *Business and Financial Mathematics*,
section 6.1: 50,000 at 5% compounded quarterly, repaid by six semi-annual instalments rounded to
the cent. The periodic rate is $1.0125^2 - 1 = 0.02515625$, computed through the series functions
of the rate conversion, so it is exact to about $10^{-27}$:

```python
from decimal import Decimal

import fairmark as fm

loan = fm.loans.amortisation_schedule(
    50000, 6, rate="0.05", compounding="quarterly", payment_frequency="semi_annual",
    amortisation_type="french", timing="ordinary", decimal_places=2, rounding_mode="half_up",
)
assert round(loan.periodic_rate, 20) == Decimal("0.02515625")  # series functions: 1e-27
first, last = loan.rows[0], loan.rows[-1]
assert first["instalment"] == Decimal("9082.24")
assert round(first["interest_portion"], 20) == Decimal("1257.8125")  # not rounded
assert round(last["instalment"], 2) == Decimal("9082.26")  # the book's final payment
assert last["outstanding_balance"] == 0
assert round(loan.final_adjustment, 4) == Decimal("0.0171")
assert round(loan.totals["interest"], 2) == Decimal("4493.46")

# Italian-style constant principal: 120,000 over 8 quarters at 6% compounded quarterly.
fixed = fm.loans.amortisation_schedule(
    120000, 8, rate="0.06", compounding="quarterly", payment_frequency="quarterly",
    amortisation_type="fixed_amortisation_schedule", timing="ordinary",
)
assert [row["principal_portion"] for row in fixed.rows] == [Decimal(15000)] * 8
assert fixed.totals["interest"] == Decimal(8100)

# Two interest-only months, then a French schedule with a balloon of 20% of the principal.
grace = fm.loans.amortisation_schedule(
    100000, 60, rate="0.048", compounding="monthly", payment_frequency="monthly",
    amortisation_type="french", timing="ordinary", principal_grace_periods=2,
    balloon_amount=20000,
)
assert grace.rows[0]["instalment"] == Decimal(400)
assert grace.rows[-1]["balloon_amount"] == Decimal(20000)
assert abs(grace.final_adjustment) < Decimal("1e-18")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `principal` | Amount borrowed, currency units, greater than 0 | required |
| `rate`, `compounding`, `payment_frequency` | Annual borrowing rate with its compounding and the instalments per year (form a) | one rate form required |
| `periodic_rate` | Rate per payment period, decimal fraction, greater than −1 (form b) | one rate form required |
| `number_of_instalments` | Full term, grace periods included, from 1 to 10000 | required |
| `timing` | `ordinary` (in arrears); `due` is not accepted by version 1 | required |
| `amortisation_type` | `french`, `fixed_amortisation_schedule` or `bullet` | required |
| `principal_grace_periods` | Interest-only periods at the start, lower than `number_of_instalments`; not with `bullet` | `0` |
| `balloon_amount` | Principal repaid with the last instalment, $0 \le B < P$; not with `bullet` | `0` |
| `rounding` | `decimal_places` (0 to 6) and `mode` (`half_up` or `half_even`), both required | none |

The rate forms are described in [`loans.instalment`](loans-instalment.md).

## Outputs

| Field | Meaning |
| --- | --- |
| `periodic_rate` | Rate per payment period used |
| `rows` | One row per period: `period`, `time`, `instalment`, `interest_portion`, `principal_portion`, `outstanding_balance`, and `balloon_amount` in the last row |
| `totals` | `instalments`, `interest` and `principal` (the principal portions add up to the principal) |
| `final_adjustment` | Last instalment less the balloon amount less the regular instalment of the last period |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Rate form | (a) annual rate with compounding and payment frequency, or (b) periodic rate | exactly one, required |
| Timing | in arrears: instalment $k$ at time $k$ | required, `ordinary` only |
| Amortisation type | `french`, `fixed_amortisation_schedule`, `bullet` | required |
| Principal grace period | 0 to $n - 1$ interest-only periods | 0, reported as default |
| Balloon amount | $0 \le B < P$, repaid with the last instalment | 0, reported as default |
| Rounding | none, or 0 to 6 decimal places with `half_up` or `half_even`, on the instalment (`french`) or the principal portion (`fixed_amortisation_schedule`) | none |
| Interest of a period | periodic rate × opening balance (compound interest), never rounded | fixed |
| Final-period adjustment | last principal portion = opening balance of the last period | always applied, reported |

The audit trail has a fixed number of steps whatever the term: the periodic rate, the regular
amount and its rounding, the recurrence stated once, the first period (and the first period
after the grace period) with the numbers substituted, the last period with the final
adjustment, and the totals. The rows carry the numbers of every period.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Timing `due`, principal 0 or negative, `number_of_instalments` 0 or above 10000, an incomplete or double rate form, rounding without a mode or with more than 6 decimal places, or a field missing or unknown |
| `FM-E003` | The periodic rate is −100% or lower |
| `FM-E020` | The balloon amount is negative, not lower than the principal, or given with `bullet` |
| `FM-E022` | The grace period is not lower than the number of instalments, or is given with `bullet` |
| `FM-E023` | With rounding, the balance would become negative before the last period, or the last principal portion or instalment would be negative (rounding too coarse for the loan) |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Each period uses one multiplication and two subtractions, so every row holds to the last of
the engine's 28 significant digits: instalment = interest portion + principal portion, closing
balance = opening balance − principal portion, and the principal portions add up to the
principal. The final outstanding balance is always exactly zero, by construction.

The instalment itself is stored with 28 significant digits. The last period pays the balance
actually left, so the final adjustment absorbs that storage error, amplified by $(1 + i)$ per
period. Measured without rounding: about $10^{-21}$ on 360 monthly periods; $1.3 \times 10^{-17}$
at 10000 periods at 0.1%; $2.9 \times 10^{-13}$ at 25% over 100 periods. It stays below
$10^{-18}$ of the principal while $(1 + i)^n \le 10^{10}$ (for example 1200 periods at 1.9%, or
470 periods at 5%); beyond that it grows with $(1 + i)^n$, to about 400 on a principal of
150,000 at 5% over 1200 periods, which is the consequence of the 28th digit of the instalment,
not of the recurrence. The present value of the instalments equals the principal within
$10^{-18}$ relative in every case. With rounding, the adjustment is dominated by the rounding
itself. The golden cases are checked to $10^{-20}$, or exactly where every value is a
terminating decimal.

## Verified against

- Two textbook worked examples, within half a cent: Valerie Watts, *Business and Financial
  Mathematics* (Fanshawe College Pressbooks), section 6.1, the 50,000 business loan; Jean-Paul
  Olivier, *Business Math: A Step-by-Step Handbook* (Lyryx Learning, 2018 Revision A), Example
  13.3A.
- Golden cases in `tests/golden/loans.amortisation_schedule/`: the ECMA-376 worked examples of
  the principal portion and an independent 50-digit reference (`scripts/reference_loans.py`),
  which is also compared row by row on random loans of the three types.
- The interest-portion and principal-portion functions of `numpy_financial` for French
  schedules without rounding.
