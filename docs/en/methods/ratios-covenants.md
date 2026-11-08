# Covenant tests (`ratios.covenants`)

Answers the question "does the company respect its covenants?". Tests a list of financial
covenants, each on the numerator and the denominator that the loan agreement defines, against
a threshold at a test date, and returns for each covenant an explicit outcome (`passed`,
`breached` or `not_tested` with a code and the reason), the tested value and the headroom.

The method does not compute the standard ratios of the other groups: a covenant is tested on
the agreement's own definitions (for example "consolidated EBITDA for the 12 months ending on
the test date, before non-recurring items"), which often differ from the reported figures. The
analyst computes the amounts under those definitions and gives them with the words of the
agreement; Fairmark echoes both in the audit trail.

Fairmark never decides what the loan agreement decides. Where agreements differ, the clause is
an explicit input with no default: the outcome when the denominator is zero or negative
(`if_denominator_not_positive`) and the rounding of the tested value (`rounding`). Every
outcome therefore traces back to a clause of the agreement.

## Covenant tests and analytical ratios

A covenant test can give a different number from the analytical ratio of the same name in
[`ratios.debt_service`](ratios-debt-service.md), and both are correct for their purpose. The
analytical ratio uses the reported aggregates under the conventions the analyst chooses
(balances, EBITDA and interest definitions). The covenant uses the definitions written in the
loan agreement, which commonly adjust EBITDA (non-recurring items, pro forma effect of
acquisitions and disposals), include or exclude items of debt (leases, shareholder loans),
cap the cash that may be netted, and fix the test period (often the 12 months ending on the
test date). The lender reads compliance only from the agreement's definitions, so the covenant
test never substitutes the analytical ratio for them.

## Formula

$$
\text{tested value} = \frac{\text{numerator}}{\text{denominator}}
\quad\text{(rounded as the agreement states, when rounding is given)}
$$

$$
\text{headroom} =
\begin{cases}
\text{threshold} - \text{tested value} & \text{maximum (at\_most, less\_than)} \\
\text{tested value} - \text{threshold} & \text{minimum (at\_least, greater\_than)}
\end{cases}
\qquad
\text{relative headroom} = \frac{\text{headroom}}{\text{threshold}}
$$

The headroom is positive on the compliant side of the threshold and negative on the breach
side, whatever the direction of the covenant. At the threshold it is zero: an inclusive operator
(`at_most`, `at_least`) passes and a strict one (`less_than`, `greater_than`) breaches.

**Precision.** The outcome is decided exactly. Without `rounding`, it is the sign of
numerator − threshold × denominator, computed exactly in integer arithmetic (the denominator is
positive at that point), so a tested value that rounds to the threshold at 28 digits is still
classified on its exact value; value and headroom are shown to the 28 significant digits of the
decimal type, not rounded to the decimals of the agreement. With `rounding`, the tested value is
rounded as the agreement states, computed from the exact quotient (no intermediate rounding),
and the outcome is the exact comparison of that rounded value with the threshold; `value` is
the rounded value and `value_unrounded` the value before rounding. When the displayed headroom
rounds to 0 at 28 significant digits (or shows the other sign) while the exact comparison
decides otherwise, a convention and an explain note say so: "displayed headroom rounds to 0 at
28 significant digits; the exact comparison decides breached".

**Zero or negative denominator.** The tested value is not computed: value and headroom are null
and the outcome is the one stated by `if_denominator_not_positive` (`breached`, `passed` or
`not_tested`), and `outcome_basis` is `if_denominator_not_positive`, so an agent can tell an
outcome set by the agreement's rule from one decided on a tested value (`tested_value`). With
`not_tested` the code is `denominator_not_positive` and the reason says
whether the denominator is zero or negative.

Note: `headroom_relative` is headroom / threshold. For a maximum covenant with a positive
tested value it is also the fraction by which the denominator can fall, the numerator
unchanged, before the covenant is breached. For a minimum covenant the fraction by which the
numerator can fall, the denominator unchanged, is headroom / value (for example a minimum
interest cover of 4 tested at 5: headroom 1, the numerator can fall by 1 / 5 = 20%, while
headroom / threshold is 25%). Fairmark does not return headroom / value as a separate field;
it follows from `headroom` and `value`.

## Example

```python
from decimal import Decimal

import fairmark as fm

ltm = "for the 12 months ending on the test date"
r = fm.ratios.covenants(
    covenants=[
        {
            "name": "net leverage",
            "test_date": "2026-12-31",
            "numerator": {"definition": "consolidated net financial debt", "amount": 1900},
            "denominator": {"definition": f"consolidated EBITDA {ltm}", "amount": 600},
            "operator": "at_most",
            "threshold": "3.5",
            "if_denominator_not_positive": "breached",
        },
        {
            "name": "interest cover",
            "test_date": "2026-12-31",
            "numerator": {"definition": f"consolidated EBITDA {ltm}", "amount": 600},
            "denominator": {"definition": f"net finance charges {ltm}", "amount": 175},
            "operator": "at_least",
            "threshold": "4",
            "if_denominator_not_positive": "passed",
        },
        {
            "name": "debt service cover",
            "test_date": "2026-12-31",
            "numerator": {"definition": f"cash flow available for debt service {ltm}", "amount": 700},
            "denominator": {"definition": f"debt service {ltm}", "amount": 0},
            "operator": "at_least",
            "threshold": "1.2",
            "if_denominator_not_positive": "not_tested",
        },
        {
            "name": "senior leverage",
            "test_date": "2026-12-31",
            "numerator": {"definition": "senior net financial debt", "amount": 3504},
            "denominator": {"definition": f"consolidated EBITDA {ltm}", "amount": 1000},
            "operator": "at_most",
            "threshold": "3.50",
            "if_denominator_not_positive": "breached",
            "rounding": {"decimal_places": 2, "mode": "half_up"},
        },
    ]
)
leverage, cover, dscr, senior = r.covenants
assert leverage["outcome"] == "passed"  # 1900 / 600 = 3.1666... <= 3.5
assert abs(leverage["headroom"] - Decimal("0.3333333333")) < Decimal("1e-10")
assert cover["outcome"] == "breached"  # 600 / 175 = 3.4285... < 4
assert cover["headroom"] < 0  # negative: on the breach side
assert dscr["outcome"] == "not_tested"  # zero debt service, as the agreement states
assert dscr["not_tested_code"] == "denominator_not_positive"
assert dscr["outcome_basis"] == "if_denominator_not_positive"
assert leverage["outcome_basis"] == "tested_value"
assert dscr["not_tested_reason"].startswith("denominator is zero")
# 3.504 rounds to 3.50 under the agreement, which passes at most 3.50
assert senior["outcome"] == "passed"
assert senior["value_unrounded"] == Decimal("3.504")
assert r.summary == {"passed": 2, "breached": 1, "not_tested": 1}
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `covenants` | List of covenants, at least one; each is tested on its own and the results keep the order | required |
| `covenants[].name` | Name of the covenant as in the loan agreement, not blank | required |
| `covenants[].test_date` | Test date, `YYYY-MM-DD`; the amounts refer to the period ending on it | required |
| `covenants[].numerator` | `definition` (the words of the agreement, not blank) and `amount` (currency units) | required |
| `covenants[].denominator` | Same as `numerator` | required |
| `covenants[].operator` | `at_most` (≤), `less_than` (<), `at_least` (≥) or `greater_than` (>) | required, no default |
| `covenants[].threshold` | In the unit of the ratio: a multiple (`3.5`) or a decimal fraction (`0.30` for 30%) | required |
| `covenants[].if_denominator_not_positive` | Outcome when the denominator is zero or negative, as the agreement states: `breached`, `passed` or `not_tested` | required, no default |
| `covenants[].rounding` | Rounding of the tested value before the comparison, as the agreement states: `decimal_places` (0 to 6) and `mode` (`half_up`: halves away from zero; `half_even`: halves to the even digit), both required | optional: no rounding |

## Outputs

| Field | Meaning |
| --- | --- |
| `covenants` | One result per covenant, in the order of the input |
| `covenants[].name`, `test_date`, `operator`, `threshold` | Echoed from the input |
| `covenants[].outcome` | `passed`, `breached` or `not_tested`, decided exactly |
| `covenants[].outcome_basis` | What decided the outcome: `tested_value`, or `if_denominator_not_positive` when the denominator is zero or negative and the input rule applies; null when the covenant is not tested because of an overflow |
| `covenants[].value` | The value compared with the threshold: numerator / denominator to 28 significant digits, or rounded as `rounding` states; null when the denominator is not positive or the covenant is not tested |
| `covenants[].value_unrounded` | With `rounding`: numerator / denominator before the rounding; null otherwise |
| `covenants[].headroom` | Distance of `value` from the threshold in the unit of the ratio, positive on the compliant side; null when `value` is null |
| `covenants[].headroom_relative` | `{"value": ..., "not_defined": ...}`: headroom / threshold, decimal fraction, same sign as the headroom |
| `covenants[].not_tested_code` | `denominator_not_positive` or `overflow`; null when the outcome is `passed` or `breached` |
| `covenants[].not_tested_reason` | Why the covenant was not tested, in words; null when the outcome is `passed` or `breached` |
| `summary` | `passed`, `breached`, `not_tested`: how many covenants have each outcome |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Definitions | numerator and denominator as defined by the loan agreement, given as amounts; no standard ratio is substituted | fixed |
| Test date | the amounts refer to the period ending on the test date | fixed |
| Operator | `at_most`, `less_than`, `at_least`, `greater_than`, stated per covenant | required |
| Denominator zero or negative | `breached`, `passed` or `not_tested`, stated per covenant; value and headroom null | required |
| Rounding | none, or `decimal_places` 0 to 6 with `half_up` or `half_even`, stated per covenant | none |
| Precision | the outcome is decided exactly: on the exact tested value, or on the value rounded as the agreement states when rounding is given; value and headroom are shown to the 28 significant digits of the decimal type, and are rounded to the decimals of the agreement only when rounding is given | fixed |
| Negative numerator | compared as computed | fixed |
| Headroom sign | positive on the compliant side, negative on the breach side | fixed |
| Relative headroom | headroom / threshold; not defined when the threshold is zero or negative, or when there is no tested value | fixed |
| Overflow | the value, the rounded value or the headroom exceeds the range of the decimal type: not tested (`overflow`), never breached, and the other covenants are tested | fixed |

Covenants are numbered from 1 in the audit trail. Each covenant adds its own test rule
("covenant 1, net leverage (2026-12-31): passes when the tested value ≤ 3.5 (maximum,
inclusive)"), its rule for a denominator that is not positive and its rounding rule, both
marked as coming from the input, a verdict step ("breached (600 / 175 at_least 4 is false,
decided exactly)"), the unrounded and rounded value when `rounding` is given and, when the
covenant is not tested, the reason; the three counts of the summary are steps too. A negative
numerator is compared as computed: a net cash position gives a negative leverage, which passes
a maximum; a negative EBITDA gives a negative cover, which breaches a minimum.

## Not covered

- Covenants on a single amount (minimum net worth, maximum capital expenditure) have no
  denominator and are not tested by this method; see the [roadmap](../project/roadmap.md).
- Equity cures are not modelled in version 1.
- Step-down schedules are expressed as one covenant per test date, each with its own threshold.
- The relative headroom on a negative threshold is not defined.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | `covenants` is empty; a blank `name` or `definition`; `rounding.decimal_places` above 6; a missing or unknown field (including `operator`, `if_denominator_not_positive`, or `rounding.mode` when `rounding` is given), or an unknown value |
| `FM-E009` | A `test_date` that is not a valid `YYYY-MM-DD` date |

## Accuracy

The outcome is exact: without rounding it compares numerator and threshold × denominator as
integers; with rounding the rounded value is computed exactly from the quotient of the two
amounts and compared with the threshold. The tested value is one decimal division, the
headroom one subtraction and the relative headroom one division, each to the 28 significant
digits of the decimal type.

## Verified against

- Golden cases in `tests/golden/ratios.covenants/`, whose expected values come from
  `scripts/reference_ratios.py` (Python fractions for the outcome and the rounding, decimal at
  50 digits for the printed values): a covenant set with one breach and the headroom of the
  others, a zero and a negative denominator under each value of `if_denominator_not_positive`,
  3.504 tested at most 3.50 with and without rounding to 2 decimals, `half_up` against
  `half_even` on a half, both operators of each direction at the threshold, negative
  numerators and non-positive thresholds; a constructed case for overflow.
- Unit tests on values that round to the threshold at 28 digits, for example
  3.0000000000000000000000000001 / 3 tested at most 1, which is breached, and on a quotient
  whose 28-digit rounding would round the wrong way at 2 decimals.
