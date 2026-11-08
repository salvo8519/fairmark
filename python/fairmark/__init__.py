"""Fairmark — deterministic valuation engine for analysts and AI agents.

    import fairmark as fm

    r = fm.dcf.fcff([100, 110, 120], "0.10", "0.02", net_debt=200)
    r.equity_value          # Decimal, exact
    r.explain_markdown()    # methodology note for the report

Agents: see ``fairmark.tools`` (function calling) and ``fairmark-mcp`` (MCP server).
"""

from importlib.metadata import PackageNotFoundError, version

from . import analysis, coc, dates, dcf, rates, ratios, tools, tv, tvm
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
    "methods",
    "rates",
    "ratios",
    "run",
    "run_json",
    "tools",
    "tv",
    "tvm",
]
