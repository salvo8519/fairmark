# Number of instalments (`loans.number_of_instalments`)

Returns how many instalments of a given amount repay a loan: the exact number of periods, the
whole number of instalments and the amount of the last instalment that brings the
outstanding balance exactly to zero. Use it when the borrower can afford a given instalment
and the question is how long the loan lasts.

## Formula

With the periodic rate $i$, the principal $P$ and the instalment $A$:

$$
n^* = \frac{-\ln\left(1 - \dfrac{i\,P'}{A}\right)}{\ln(1 + i)}
\qquad
P' =
\begin{cases}
P & \text{in arrears} \\
P / (1 + i) & \text{in advance}
\end{cases}
\qquad
n^* = \frac{P}{A} \text{ when } i = 0
$$

The number of instalments is $N = \lceil n^* \rceil$. The last instalment is the amount owed on
its payment date, from the recurrence of the outstanding balance:

$$
b_1 = P\,(1 + i) \text{ (arrears) or } P \text{ (advance)}, \qquad
b_k = (b_{k-1} - A)(1 + i), \qquad
\text{final instalment} = b_N
$$

The periodic rate is derived as in [`loans.instalment`](loans-instalment.md): an annual
borrowing rate with its compounding and payment frequency, or a periodic rate.

## Example

A loan of 3,000 at 2% a quarter repaid by quarterly instalments of 800 needs four
instalments, the last one lower than the others:

```python
from decimal import Decimal

import fairmark as fm

term = fm.loans.number_of_instalments(3000, 800, periodic_rate="0.02", timing="ordinary")
assert term.number_of_instalments == 4
assert term.final_instalment == Decimal("750.01008")  # (((3000 x 1.02 - 800) x 1.02 - 800) x 1.02 - 800) x 1.02
assert abs(term.exact_number_of_periods - Decimal("3.936929191448856778307291")) < Decimal("1e-20")

# Monthly instalments of 500 in advance, 7% effective annual rate.
advance = fm.loans.number_of_instalments(
    15000, 500, rate="0.07", compounding="annual", payment_frequency="monthly", timing="due"
)
assert advance.number_of_instalments == 33
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `principal` | Amount borrowed, currency units, greater than 0 | required |
| `rate`, `compounding`, `payment_frequency` | Annual borrowing rate with its compounding and the instalments per year (form a) | one rate form required |
| `periodic_rate` | Rate per payment period, decimal fraction, greater than −1 (form b) | one rate form required |
| `instalment` | Constant instalment, currency units | required |
| `timing` | `ordinary` (in arrears) or `due` (in advance) | required |

A balloon amount is not accepted by this method version.

## Outputs

| Field | Meaning |
| --- | --- |
| `periodic_rate` | Rate per payment period used |
| `exact_number_of_periods` | $n^*$ |
| `number_of_instalments` | $N = \lceil n^* \rceil$, from 1 to 10000 |
| `final_instalment` | Amount of the last instalment, which brings the balance to zero |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Rate form | (a) annual rate with compounding and payment frequency, or (b) periodic rate | exactly one, required |
| Timing | `ordinary`, `due` | none: must be chosen |
| Whole number of periods | an $n^*$ within $10^{-20}$ of an integer is taken as that integer; $N$ is then confirmed by the balance recurrence: the first instalment whose amount owed exceeds $A$ by at most $10^{-20}$ of the principal | fixed |
| Final instalment | amount owed on its payment date, balance exactly zero: lower than $A$ when $n^*$ is not whole, above $A$ by at most $10^{-20}$ of the principal when $n^*$ is taken as a whole number | fixed |
| Rate zero | $n^* = P / A$, shown as a separate step | applied when $i = 0$ |

The tolerance on whole numbers and the recurrence exist so that an instalment computed to full
precision by [`loans.instalment`](loans-instalment.md) gives back its number of instalments,
instead of one more instalment of a fraction of a cent.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Principal 0 or negative, more than 10000 instalments needed, an incomplete or double rate form, a `balloon_amount` field, or a field missing or unknown |
| `FM-E003` | The periodic rate is −100% or lower |
| `FM-E021` | The instalment is 0 or below, or does not exceed $i\,P'$, the interest of the first period, whatever the sign of the rate: the balance never reaches zero |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

$n^*$ is computed as $q \times \dfrac{\ln(1 - i q)/(-i q)}{\ln(1 + i)/i}$ with $q = P'/A$: the
ratios $\ln(1 + x)/x$ are summed from their series when $|x| < 0.01$, so small rates of either
sign keep every digit; $n^*$ agrees with an independent 50-digit reference to about
$10^{-24}$, including rates of $\pm 10^{-9}$ and $-10^{-11}$ per period. The number of instalments
and the final instalment do not depend on that accuracy: $N$ is confirmed by the balance
recurrence, which uses only multiplication and subtraction and holds to the last of the
engine's 28 significant digits. The golden cases are checked to $10^{-20}$.

## Verified against

- Golden cases in `tests/golden/loans.number_of_instalments/`, from an independent 50-digit
  reference (`scripts/reference_loans.py`), including the instalment of the ECMA-376 worked
  example, which gives back exactly 10 instalments, and small negative rates over 1200 and
  10000 periods.
- The number-of-periods function of `numpy_financial` on random loans, both timings.
