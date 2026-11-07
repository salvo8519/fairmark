# SPEC-0001 — Complete DCF for private companies

| | |
| --- | --- |
| Status | Draft — waiting for the maintainer's methodology sheet and practitioner cases |
| Milestone | M2 |
| Owner | @salvo8519 |
| Methodology sources | Principi Italiani di Valutazione (PIV); practitioner models (to be provided); standard corporate-finance textbooks for CAPM/WACC |
| Related | ADR 0001–0003, `dcf.fcff` (preview, method v1) |

## Problem
Analysts valuing private companies start from statutory financial statements and a business
plan, not from a ticker. Today `dcf.fcff` needs free cash flows already computed: the most
error-prone part of the work (reclassification, FCFF build-up, cost of capital) happens
outside Fairmark, often inside an LLM or a fragile spreadsheet.

## Scope
**In scope:** reclassification of statutory accounts, FCFF and FCFE build-up from a business
plan, cost of capital (CAPM, beta relevering, WACC with explicit premia), terminal value
(Gordon and exit multiple, configurable timing), equity bridge, deterministic sensitivity grid.

**Out of scope:** market data retrieval (betas, risk-free rates, premia are inputs), IFRS 16
special handling beyond an explicit lease-liability input, real estate (separate spec).

## User stories
- As an analyst, I want to pass a reclassified business plan and get FCFF per year with every
  line explained, so that I can attach the build-up to my report.
- As an analyst, I want WACC computed from its components with the premia I chose, so that the
  discount rate is traceable.
- As an AI agent, I want one call that goes from business plan to equity value, so that I
  never compute an intermediate number myself.

## Requirements (EARS)
- **R1** — WHEN given statutory income statement and balance sheet items (art. 2424–2425 c.c.
  layout), THE ENGINE SHALL produce a reclassified P&L (value added) and a functional balance
  sheet, including net financial position (PFN), with every aggregate explained.
- **R2** — WHEN given a multi-year business plan (EBIT, D&A, capex, change in operating working
  capital, tax rate inputs), THE ENGINE SHALL compute FCFF per year as
  EBIT × (1 − t) + D&A − capex − ΔNWC, and report each component in `explain`.
- **R3** — THE ENGINE SHALL compute notional taxes on EBIT using explicit tax inputs; the
  treatment of IRES and IRAP SHALL be a documented parameter.
- **R4** — WHEN given risk-free rate, equity risk premium, unlevered beta, target D/E, tax rate,
  size premium and specific risk premium, THE ENGINE SHALL compute the relevered beta (method
  selectable: Hamada, Harris-Pringle), cost of equity, after-tax cost of debt and WACC.
- **R5** — THE ENGINE SHALL support a terminal value by Gordon growth or by exit multiple, and
  the discounting time of the terminal value SHALL be an explicit parameter.
- **R6** — THE ENGINE SHALL bridge enterprise value to equity value with explicit items: PFN,
  debt-like items (e.g. TFR, lease liabilities, if chosen), minorities, surplus assets.
- **R7** — WHEN requested, THE ENGINE SHALL compute a WACC × g sensitivity grid in a single call,
  deterministically, with the grid definition echoed in the result.
- **R8** — IF any input needed by a requested calculation is missing or inconsistent (e.g.
  g ≥ WACC, negative D/E), THEN THE ENGINE SHALL return a typed error and never default.
- **R9** — FCFE build-up (net borrowing, interest after tax) SHALL be available with the
  cost of equity as discount rate.

## Conventions and parameters
| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Cash-flow timing | end / mid period | end | practice varies — to confirm |
| Terminal value discounting | t = n / t = n − 0.5 under mid period | t = n | to confirm |
| IRAP in notional taxes | included / excluded | to confirm | maintainer sheet |
| TFR in equity bridge | debt-like / operating | to confirm | maintainer sheet |
| Beta relevering | Hamada / Harris-Pringle | Hamada | textbook |

## Acceptance criteria
- **AC1** (R1) — Reclassification of 3 anonymised practitioner cases matches within 0.01 €.
- **AC2** (R2, R3) — FCFF per year for the same cases matches the practitioner models within 1e-6.
- **AC3** (R4) — WACC for textbook and practitioner cases matches within 1e-12.
- **AC4** (R5, R6) — Equity value for each practitioner case matches within 0.01 €.
- **AC5** (R7) — Every grid cell equals a direct single-point run (property test).
- **AC6** (R8) — Each invalid-input scenario listed in plan.md returns its documented error code.
- **AC7** — Methodology pages published for every new method, reference regenerated.

## Interfaces
- New method ids (tentative): `statements.reclassify`, `dcf.fcff_build`, `coc.wacc`,
  `dcf.valuation` (end-to-end), `dcf.sensitivity`, `dcf.fcfe`.
- `dcf.fcff` stays available; end-to-end valuation reuses it.

## Open questions
- [ ] Maintainer methodology sheet: formulas and conventions used in practice (IRES/IRAP, PFN
      definition, TFR, leases, mid-year).
- [ ] 3–5 anonymised practitioner cases with full inputs and outputs.
- [ ] Minimum statutory layout to support first: full (ordinary) or abbreviated accounts?
