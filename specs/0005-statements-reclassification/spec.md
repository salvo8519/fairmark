# SPEC-0005 — Reclassification of Italian statutory accounts and FCFF build-up

| | |
| --- | --- |
| Status | Draft — waiting for the maintainer's methodology sheet and practitioner cases |
| Milestone | M2 |
| Owner | @salvo8519 |
| Formula basis | Statutory layout of the Civil Code (art. 2424–2425) and common reclassification practice; practitioner models (to be provided) |
| Related | SPEC-0001 (consumes FCFF), SPEC-0003 (taxes) |

## Problem
Analysts start from statutory accounts, not from free cash flows. Reclassification and FCFF
build-up are the most error-prone steps and today happen outside Fairmark.

## Scope
**In scope:** reclassified P&L (value added), functional balance sheet, net financial position,
FCFF and FCFE build-up from a business plan with explicit IRES and IRAP treatment.
**Out of scope:** XBRL parsing (an adapter concern), consolidated accounts.

## Requirements (EARS)
- **R1** — WHEN given statutory items, THE ENGINE SHALL produce the reclassified P&L and the
  functional balance sheet with every aggregate explained.
- **R2** — WHEN given a business plan, THE ENGINE SHALL compute FCFF = EBIT × (1 − t) + D&A −
  capex − ΔNWC per year, with taxes from explicit IRES and IRAP inputs.
- **R3** — THE ENGINE SHALL compute FCFE from FCFF, interest after tax and net borrowing.

## Open questions
- [ ] Maintainer methodology sheet: PFN definition, TFR, leases, IRAP base.
- [ ] Three to five anonymised practitioner cases with full inputs and outputs.
- [ ] First layout to support: ordinary or abbreviated accounts.
