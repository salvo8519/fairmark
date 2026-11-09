"""Input examples of the tool definitions, copied from golden cases (issue #264, ADR 0014 G1).

`TABLE` names, for every method, the golden case whose input is the minimal example and,
where a larger input shows fields the minimal one does not, the golden case whose input is
the maximal example. This script copies those inputs into
`python/fairmark/tool_examples.json`, which ships with the Python package (golden cases do
not), and records the source of each example next to it.

    python scripts/tool_examples.py           # write python/fairmark/tool_examples.json
    python scripts/tool_examples.py --check   # fail if the file is stale

The examples are adapter output only: the method input schemas (`schemas/`) are unchanged.
`python/tests/test_tool_definitions.py` checks that every example validates against its
method's schema and runs through the engine without error.

YAML timestamps are kept as the ISO 8601 strings they are written as, and YAML floats as the
decimal strings they are written as (the exact representation the tools recommend).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import yaml

ROOT = Path(__file__).resolve().parents[1]
GOLDEN = ROOT / "tests" / "golden"
OUTPUT = ROOT / "python" / "fairmark" / "tool_examples.json"

# method id -> (golden case of the minimal example, golden case of the maximal example or None)
TABLE: dict[str, tuple[str, str | None]] = {
    "analysis.goal_seek": ("irr-via-npv", "dcf-implied-discount-rate"),
    "analysis.scenarios": ("zero-base", "dcf-three-cases"),
    "analysis.sensitivity": ("wacc-one-way", "wacc-g-5x5"),
    "analysis.tornado": ("npv-tie", "dcf-four-drivers"),
    "coc.beta_peers": ("three-peers-mean-harris-pringle", "five-peers-median"),
    "coc.beta_relever": ("hamada-relever", "harris-pringle-debt-beta"),
    "coc.cost_of_debt": ("spread", None),
    "coc.cost_of_equity": ("capm-basic", "capm-with-premia"),
    "coc.wacc": ("target-debt-weight", "target-debt-to-equity"),
    "dates.schedule": ("quarterly-from-1-january", "day-count-act-360"),
    "dates.year_fraction": ("act-360", None),
    "dcf.fcfe": (
        "flows-only-single-cost-of-equity",
        "three-stage-cost-of-equity-per-period-bridge",
    ),
    "dcf.fcff": ("flows-only-single-wacc", "bridge-every-line"),
    "dcf.free_cash_flows": ("damodaran-boeing-illustration-15-7", "amounts-form-with-adjustments"),
    "loans.amortisation_schedule": ("french-annual", "french-grace-balloon"),
    "loans.instalment": ("periodic-rate-form", "balloon-arrears"),
    "loans.number_of_instalments": ("non-integer-arrears", "non-integer-advance"),
    "rates.convert": ("monthly-to-annual", None),
    "rates.real_nominal": ("to-real", None),
    "ratios.capital_structure": ("closing-balances", "average-balances"),
    "ratios.covenants": ("display-at-threshold", "one-breach"),
    "ratios.debt_service": ("no-debt-schedule", "level-cfads"),
    "ratios.liquidity": ("closing-balances", "average-balances"),
    "ratios.profitability": ("closing-balances", "leverage-gross-debt-average"),
    "ratios.working_capital": ("closing-365", "average-360"),
    "series.project": ("constant-period-1", "schedule-quarterly-end-of-period"),
    "statements.reorganise": ("balance-sheet-abridged", "adjustment-non-operating-asset"),
    "tv.compare": ("equity-basis", "seven-techniques"),
    "tv.exit_multiple": ("pe-equity-basis", "ev-ebitda-implied-growth"),
    "tv.gordon": ("damodaran-fcff-from-roc-table", None),
    "tv.h_model": ("declining-growth", None),
    "tv.limited_horizon": ("level-flow-ten-periods", "growing-flow-with-residual"),
    "tv.liquidation": ("separable-assets", None),
    "tv.normalised_income": ("no-growth", "with-growth"),
    "tv.value_driver": ("damodaran-g-002-ronic-006", None),
    "tvm.annuity": ("ordinary", "growing"),
    "tvm.future_value": ("present-value-only", "ecma-376-present-and-payments-due"),
    "tvm.irr": ("conventional", None),
    "tvm.mirr": ("ecma-376-three-years", None),
    "tvm.npv": ("five-flows", None),
    "tvm.perpetuity": ("growing", None),
    "tvm.present_value": ("flat-rate-end", "curve-mid"),
    "tvm.xirr": ("ecma-376-example", None),
    "tvm.xnpv": ("ecma-376-example", "explicit-valuation-date"),
}


class _Loader(yaml.SafeLoader):
    """Safe loader that keeps timestamps and floats as the text they are written as."""


_Loader.yaml_implicit_resolvers = {
    key: [(tag, regexp) for tag, regexp in resolvers if tag != "tag:yaml.org,2002:timestamp"]
    for key, resolvers in yaml.SafeLoader.yaml_implicit_resolvers.items()
}
_Loader.add_constructor(
    "tag:yaml.org,2002:float",
    lambda loader, node: loader.construct_scalar(node),  # type: ignore[arg-type]
)


def golden_input(method: str, case: str) -> dict[str, Any]:
    """The input of `tests/golden/<method>/<case>.yaml`, which must be a successful case."""
    path = GOLDEN / method / f"{case}.yaml"
    data = yaml.load(path.read_text(encoding="utf-8"), Loader=_Loader)
    if data.get("method") != method or "expect_error" in data:
        raise SystemExit(f"{path}: not a successful golden case of {method}")
    value: dict[str, Any] = data["input"]
    return value


def build() -> dict[str, Any]:
    out: dict[str, Any] = {}
    for method in sorted(TABLE):
        minimal, maximal = TABLE[method]
        entry: dict[str, Any] = {}
        for kind, case in (("minimal", minimal), ("maximal", maximal)):
            if case is not None:
                entry[kind] = {
                    "source": f"tests/golden/{method}/{case}.yaml",
                    "input": golden_input(method, case),
                }
        out[method] = entry
    return out


def render() -> str:
    return json.dumps(build(), ensure_ascii=False, indent=2) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--check", action="store_true", help="fail if the file is stale")
    args = parser.parse_args(argv)
    text = render()
    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != text:
            print(f"{OUTPUT.relative_to(ROOT)} is stale: run python scripts/tool_examples.py")
            return 1
        print("Tool examples are current.")
        return 0
    OUTPUT.write_text(text, encoding="utf-8")
    print(f"Wrote {OUTPUT.relative_to(ROOT)} ({len(TABLE)} methods).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
