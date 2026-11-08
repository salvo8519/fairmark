"""Fairmark — deterministic valuation engine for analysts and AI agents.

    import fairmark as fm

    r = fm.dcf.fcff(
        [fm.dcf.given([100, 110, 120])],
        cash_flow_terms="nominal", discount_rate_terms="nominal",
        wacc="0.10", timing="end_of_period",
        terminal=fm.dcf.gordon("0.02", first_flow="last_flow_grown"),
        net_financial_position={"gross_debt": 200, "cash": 0},
    )
    r.equity_value          # Decimal, exact
    r.explain_markdown()    # methodology note for the report

Agents: see ``fairmark.tools`` (function calling) and ``fairmark-mcp`` (MCP server).
"""

from importlib.metadata import PackageNotFoundError, version

from . import analysis, coc, dates, dcf, loans, rates, ratios, series, statements, tools, tv, tvm
from ._core import FairmarkError, Result, Step, input_schema, methods, run, run_json
from ._engine import ENGINE_VERSION

try:
    __version__ = version("fairmark")
except PackageNotFoundError:  # pragma: no cover - running from a source tree
    __version__ = ENGINE_VERSION

__all__ = [
    "ENGINE_VERSION",
    "FairmarkError",
    "Result",
    "Step",
    "__version__",
    "analysis",
    "coc",
    "dates",
    "dcf",
    "input_schema",
    "loans",
    "methods",
    "rates",
    "ratios",
    "run",
    "run_json",
    "series",
    "statements",
    "tools",
    "tv",
    "tvm",
]
