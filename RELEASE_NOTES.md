# Release notes

What changes for users in each version of Fairmark, in plain language. The technical history
of every change is in the [changelog](https://salvo8519.github.io/fairmark/project/changelog/).
On the documentation site, the first section lists the changes not yet released.

<!-- release notes start -->

## Fairmark 0.1.0-alpha.1 — 2026-11-24

The first public release of Fairmark: a deterministic valuation engine for financial analysts
and AI agents. Every result is computed in exact decimal arithmetic, is the same on every
platform for the same input and version, and comes with the formula, the conventions and every
intermediate step, so it can be checked line by line. The engine performs no input or output
of its own and Fairmark sends no data over the network.

This is an alpha release. Inputs and results may still change before version 0.1.0; every
change is recorded in these notes and every method keeps a version of its own.

### Methods

44 methods. 32 are available: each has an approved specification, test cases whose expected
values come from sources independent of the implementation, an independent numerical audit and
a code review. 12 are in preview: implemented, tested against the same kind of test cases and
documented, but not yet independently verified. Every result states the status of the method
that produced it.

- **Time value of money** (available): net present value, present and future value, IRR, XIRR,
  XNPV, MIRR, annuities and perpetuities; day counts (Actual/365, Actual/360, Actual/Actual ISDA,
  30/360, 30E/360), rate conversion between compounding frequencies and between real and nominal
  terms. When a cash flow profile has more than one internal rate of return, the result reports
  the ambiguity and the intervals where the rates lie instead of returning one of them.
- **Cost of capital** (available): cost of equity with the CAPM and explicit country, size and
  specific premia (a specific premium requires a written justification), unlevered and relevered
  betas (Hamada or Harris-Pringle), the beta of a group of comparable companies, the cost of debt
  and the WACC.
- **Terminal value** (available): stable growth (Gordon), value driver with return on new
  capital, H-model, exit multiples with the growth they imply, finite horizon, normalised income
  and liquidation value, and a comparison of several techniques on the same rate and basis.
- **Discounted cash flow** (preview): DCF on free cash flows to the firm and to equity, with a
  plan in up to three stages (given flows, constant growth, growth declining to its long-term
  rate), one rate or one rate per year, end-of-period or mid-year timing, any of the terminal
  value techniques above, and the bridge from the value of operations to equity value with every
  item listed: net financial position, debt-like items and minority interests, each with its
  measurement basis. A plan at constant prices can be discounted at a nominal rate, or the
  reverse, by giving expected inflation; without it, mixed terms are refused.
- **Free cash flows from a business plan** (preview): from EBIT, taxes, depreciation and
  amortisation, capital expenditure and working capital to the free cash flow to the firm, and
  with interest and net borrowing to the free cash flow to equity. The treatment of a negative
  EBIT is an explicit choice; with the "zero" choice, the tax saving on interest is limited to
  the taxable income available.
- **Financial statements** (preview): reorganisation of the balance sheet and of the income
  statement by nature, from the statutory aggregates of an Italian filing (ordinary or abridged
  layout) or from totals already reclassified, into invested capital, net financial position,
  EBITDA, operating profit and the subtotals of IFRS 18, with every total checked and every
  amount taken as zero stated.
- **Ratios** (preview): profitability, capital structure, liquidity, working capital and debt
  service (interest cover, net financial position to EBITDA, DSCR and LLCR), and covenant tests
  on the definitions of a loan agreement. Each ratio is computed as soon as its own figures are
  given; a ratio that cannot be computed or is not meaningful is reported with the reason.
- **Loans** (available): constant instalment, number of instalments and full amortisation
  schedule (constant instalment, constant principal, bullet, interest-only periods, balloon),
  with rounding to the cent as banks apply it.
- **Periods and series** (preview): period schedules on calendar dates and projections of a base
  value with constant, linear or per-period growth.
- **Analysis** (available): goal seek, one-way and two-way sensitivity tables, scenarios and
  tornado ranking on any method, with every evaluation in the audit trail.

### Results an agent can act on

- One result envelope for every method: the value, the explanation (formula, conventions,
  steps), the diagnostics (outputs not computed and what they need, warnings with stable codes
  FM-W001 to FM-W017, checks) and the metadata (engine and method versions, method status, a
  hash of the input, the defaults applied).
- A refused input returns every error in one answer, in a fixed order, each with the field to
  correct as a JSON pointer, the accepted values and how to fix it; a misspelt field name comes
  with the name it probably meant.
- Numbers are exact decimals, preferably written as strings. A number with more than 28 decimal
  places is rounded and the result says so; a value that arrived as a binary floating-point
  number is flagged.
- Every method publishes the schema of its input and of its result for each method version.

### Channels

- **Python** 3.12 or newer: the `fairmark` package, with helpers for every area.
- **Command line**: the `fairmark` program (`methods`, `schema`, `run`, `verify`), JSON in and
  JSON out.
- **AI agents**: an MCP server (`fairmark-mcp`, over stdio or HTTP, for example for n8n) and
  function-calling tools for Anthropic, OpenAI, Google Gemini, OpenRouter and LiteLLM, for
  open-weight models on Ollama, vLLM or LM Studio, and for LangChain.
- **Documentation** in English and Italian, with a methodology page for every method.

### Install

Packages are published for Linux (x86_64 and ARM64, glibc and musl), macOS (Intel and Apple
silicon) and Windows (x86_64). An alpha release is installed only when asked for by version:

```bash
pip install "fairmark==0.1.0a1"                                  # Python
pip install "fairmark[mcp]==0.1.0a1"                             # with the MCP server
cargo install fairmark-cli --version 0.1.0-alpha.1 --locked      # command line
```

Fairmark is a calculation tool, not an advisor: decisions taken with its results belong to the
user, who verifies them before relying on them.

