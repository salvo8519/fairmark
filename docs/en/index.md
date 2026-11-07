---
hide:
  - navigation
  - toc
---

<div class="fm-hero" markdown>

![Fairmark](assets/brand/mark.svg){ width="96" }

# Fairmark

<p class="fm-tagline">Fair value, hallmarked.</p>

The deterministic valuation engine for analysts and AI agents.
Your agent reasons and writes the report. Fairmark does the math — exactly, with an audit trail.

[Get started](getting-started/quickstart.md){ .md-button .md-button--primary }
[Use it with an agent](getting-started/first-valuation-with-an-agent.md){ .md-button }

</div>

```bash
pip install fairmark
```

```python
import fairmark as fm

r = fm.dcf.fcff([100, 110, 120], discount_rate="0.10", terminal_growth="0.02", net_debt=200)
r.equity_value        # Decimal('1221.4876033057851239669421487') — exact, reproducible
r.explain_markdown()  # every step, ready for your valuation report
```

<div class="grid cards fm-features" markdown>

-   __Deterministic__

    ---

    Exact decimal arithmetic and fixed solvers. Same input, same output, to the last digit,
    on every platform.

    [The determinism contract](concepts/determinism.md)

-   __Auditable__

    ---

    Every result explains itself: formula, conventions and each intermediate step,
    plus a fingerprint to reproduce it later.

    [The audit trail](concepts/audit-trail.md)

-   __Agent-native__

    ---

    One engine for Python, function calling, MCP, the CLI and Agent Skills for Claude Code
    and Codex. The model never does the arithmetic.

    [Agent guides](guides/function-calling.md)

-   __Built for private companies__

    ---

    Inputs come from financial statements and business plans, not tickers. European
    valuation practice is first-class.

    [Methods](methods/index.md)

</div>

## How it fits in your workflow

```mermaid
flowchart LR
    D[Financial statements<br/>business plan] --> L[LLM extracts<br/>the inputs]
    L --> F[Fairmark computes<br/>exact + explain]
    F --> R[LLM writes the report<br/>citing every step]
```

!!! note "Status: pre-alpha"
    Fairmark is under active development. NPV, IRR and DCF (FCFF) are available today;
    see the [roadmap](project/roadmap.md) for what comes next.
