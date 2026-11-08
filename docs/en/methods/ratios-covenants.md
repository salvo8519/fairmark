# Covenant tests (`ratios.covenants`)

Answers the question "does the company respect its covenants?". Tests a list of financial
covenants, each on the numerator and the denominator that the loan agreement defines, against
a threshold at a test date, and returns for each covenant an explicit outcome (`passed`,
`breached` or `not_tested` with the reason), the tested value and the headroom.

The method does not compute the standard ratios of the other groups: a covenant is tested on
the agreement's own definitions (for example "consolidated EBITDA for the 12 months ending on
the test date, before non-recurring items"), which often differ from the reported figures. The
analyst computes the amounts under those definitions and gives them with the words of the
agreement; Fairmark echoes both in the audit trail.

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

The outcome is decided exactly; value and headroom are shown to the 28 significant digits of
the decimal type, not rounded to the decimals of the agreement. The decision is the sign of
numerator − threshold × denominator, computed exactly in integer arithmetic (the denominator
is positive at that point), so a tested value that rounds to the threshold at 28 digits is
still classified on its exact value.

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
        },
        {
            "name": "interest cover",
            "test_date": "2026-12-31",
            "numerator": {"definition": f"consolidated EBITDA {ltm}", "amount": 600},
            "denominator": {"definition": f"net finance charges {ltm}", "amount": 175},
            "operator": "at_least",
            "threshold": "4",
        },
        {
            "name": "debt service cover",
            "test_date": "2026-12-31",
            "numerator": {"definition": f"cash flow available for debt service {ltm}", "amount": 700},
            "denominator": {"definition": f"debt service {ltm}", "amount": 0},
            "operator": "at_least",
            "threshold": "1.2",
        },
    ]
)
leverage, cover, dscr = r.covenants
assert leverage["outcome"] == "passed"  # 1900 / 600 = 3.1666... <= 3.5
assert abs(leverage["headroom"] - Decimal("0.3333333333")) < Decimal("1e-10")
assert cover["outcome"] == "breached"  # 600 / 175 = 3.4285... < 4
assert cover["headroom"] < 0  # negative: on the breach side
assert dscr["outcome"] == "not_tested"  # zero denominator: never a breach
assert dscr["not_tested_code"] == "zero_denominator"
assert dscr["not_tested_reason"].startswith("denominator is zero")
assert r.summary == {"passed": 1, "breached": 1, "not_tested": 1}
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `covenants` | List of covenants, at least one; each is tested on its own and the results keep the order | required |
| `covenants[].name` | Name of the covenant as in the loan agreement, not blank | required |
| `covenants[].test_date` | Test date, `YYYY-MM-DD`; the amounts refer to the period ending on it | required |
| `covenants[].numerator` | `definition` (the words of the agreement, not blank) and `amount` (currency units) | required |
| `covenants[].denominator` | Same as `numerator`; zero means not tested, negative is refused | required |
| `covenants[].operator` | `at_most` (≤), `less_than` (<), `at_least` (≥) or `greater_than` (>) | required, no default |
| `covenants[].threshold` | In the unit of the ratio: a multiple (`3.5`) or a decimal fraction (`0.30` for 30%) | required |

## Outputs

| Field | Meaning |
| --- | --- |
| `covenants` | One result per covenant, in the order of the input |
| `covenants[].name`, `test_date`, `operator`, `threshold` | Echoed from the input |
| `covenants[].outcome` | `passed`, `breached` or `not_tested`, decided exactly |
| `covenants[].value` | Tested value, numerator / denominator, to 28 significant digits; null when not tested |
| `covenants[].headroom` | Distance from the threshold in the unit of the ratio, positive on the compliant side, to 28 significant digits; null when not tested |
| `covenants[].headroom_relative` | `{"value": ..., "not_defined": ...}`: headroom / threshold, decimal fraction, same sign as the headroom |
| `covenants[].not_tested_code` | `zero_denominator` or `overflow`; null when tested |
| `covenants[].not_tested_reason` | Why the covenant was not tested, in words; null when tested |
| `summary` | `passed`, `breached`, `not_tested`: how many covenants have each outcome |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Definitions | numerator and denominator as defined by the loan agreement, given as amounts; no standard ratio is substituted | fixed |
| Test date | the amounts refer to the period ending on the test date | fixed |
| Operator | `at_most`, `less_than`, `at_least`, `greater_than`, stated per covenant | required |
| Precision | the outcome is decided exactly; value and headroom are shown to the 28 significant digits of the decimal type, not rounded to the decimals of the agreement | fixed |
| Negative numerator | compared as computed | fixed |
| Headroom sign | positive on the compliant side, negative on the breach side | fixed |
| Relative headroom | headroom / threshold; not defined when the threshold is zero or negative, or the covenant is not tested | fixed |
| Not tested | the denominator is zero (`zero_denominator`), or the value or headroom exceeds the range of the decimal type (`overflow`): the covenant is not tested, with the reason, never breached, and the other covenants are tested | fixed |

Covenants are numbered from 1 in the audit trail. Each covenant adds its own test rule
("covenant 1, net leverage (2026-12-31): passes when the tested value ≤ 3.5 (maximum,
inclusive)"), a verdict step ("breached (600 / 175 at_least 4 is false, decided exactly)") and,
when it is not tested, the reason; the three counts of the summary are steps too. A negative
numerator is compared as computed: a net cash position gives a negative leverage, which passes
a maximum; a negative EBITDA gives a negative cover, which breaches a minimum.

## Not yet decided

The following cases depend on the terms of each loan agreement and are left to the maintainer
before Fairmark models them. Until then the method refuses or leaves them undefined rather than
choose.

- A negative denominator (for example net debt over a negative EBITDA) is refused with
  `FM-E001`: some agreements treat it as a breach, others differently.
- An agreement that rounds the ratio to a number of decimals before the comparison is not
  modelled: the outcome is decided on the exact value.
- The relative headroom on a negative threshold is not defined.
- Covenants on a single amount (minimum net worth, maximum capital expenditure), equity cures
  and step-down schedules are not modelled; a step-down is one entry per test date with its own
  threshold.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | `covenants` is empty; a blank `name` or `definition`; a negative denominator; a missing or unknown field, or an unknown `operator` |
| `FM-E009` | A `test_date` that is not a valid `YYYY-MM-DD` date |

## Accuracy

The outcome is exact: it compares numerator and threshold × denominator as integers, with no
rounding. The tested value is one decimal division, the headroom one subtraction and the
relative headroom one division, each to the 28 significant digits of the decimal type.

## Verified against

- Golden cases in `tests/golden/ratios.covenants/`, whose expected values come from
  `scripts/reference_ratios.py` (Python decimal at 50 digits): a covenant set with one breach
  and the headroom of the others, a zero denominator, both operators of each direction at the
  threshold, negative numerators and non-positive thresholds; a constructed case for overflow.
- Unit tests on values that round to the threshold at 28 digits, for example
  3.0000000000000000000000000001 / 3 tested at most 1, which is breached.
