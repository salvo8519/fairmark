# SPEC-0003 — Cost of capital: CAPM, beta, cost of debt and WACC

| | |
| --- | --- |
| Status | Implementing — approved by the maintainer on 2026-11-07 |
| Milestone | M1 |
| Owner | @salvo8519 |
| Formula basis | Commonly adopted CAPM, Hamada and Harris-Pringle relevering, WACC; cross-checked against widely used implementations and published worked examples |
| Related | ADR 0008; SPEC-0002 (rates); used by SPEC-0001 and SPEC-0004 |

## Problem
The discount rate drives every income-based valuation and is the input most often assembled
by hand. For private companies, practice adds premia (size, company-specific, country) and
relevers comparable betas to a target capital structure. Each choice must be visible and
reproducible.

## Scope
**In scope:** cost of equity by CAPM with explicit premia; unlevering and relevering beta
(Hamada, Harris-Pringle, with optional debt beta); cost of debt before and after tax; WACC with
target weights; beta statistics from a peer list (mean, median).
**Out of scope:** estimating betas from price series; market data retrieval; build-up models
without CAPM (possible later spec).

## Requirements (EARS)
- **R1** — WHEN given risk-free rate, levered beta and equity risk premium, THE ENGINE SHALL
  return ke = rf + β_L × ERP + CRP × λ + size premium + specific premium, each premium an
  explicit input reported in `explain` (zero only when given as zero or documented as default).
- **R2** — WHEN given an unlevered beta, a target D/E and a tax rate, THE ENGINE SHALL relever by
  the selected formula: Hamada β_L = β_U [1 + (1 − t) D/E]; Harris-Pringle β_L = β_U (1 + D/E);
  with debt beta β_D: β_L = β_U + (β_U − β_D)(1 − t) D/E (Hamada) or (β_U − β_D) D/E
  (Harris-Pringle). The inverse SHALL unlever a levered beta.
- **R3** — WHEN given a list of peer betas with their D/E and tax rates, THE ENGINE SHALL
  unlever each, return mean and median, and relever the selected statistic to the target D/E.
- **R4** — THE ENGINE SHALL compute the pre-tax cost of debt either directly or as risk-free
  rate plus spread, and the after-tax cost of debt as kd × (1 − t).
- **R5** — WHEN given ke, kd, tax rate and target weights (D/E or D/(D+E)), THE ENGINE SHALL
  return WACC = E/V × ke + D/V × kd × (1 − t) with weights reported.
- **R6** — IF a weight is negative, D/(D+E) ≥ 1 (or a D/E so large that the equity weight
  rounds to zero), a country risk premium or country exposure is negative, or a beta formula is
  not one of the listed ones, THEN THE ENGINE SHALL return a typed error.
- **R7** — Every result that contains rates SHALL report that they are annual effective rates,
  and every beta conversion SHALL report its formula, in `explain.conventions`.

## Conventions and parameters
| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Relevering formula | hamada, harris_pringle | required | Damodaran; Koller |
| Debt beta | value or omitted | omitted means 0, documented | Koller |
| Tax rate for the tax shield | input | required | PIV |
| Premia (size, specific, country × λ) | inputs | 0 when omitted, documented in the schema | PIV; Damodaran |
| Peer statistic | mean, median | required | practice |

## Acceptance criteria
- **AC1** (R1, R2, R5) — Published worked examples (Damodaran's public lecture slides: Hamada
  relevering and unlevering) match at their published precision; cost of equity and WACC
  examples whose expected values are closed forms written out in the golden file match to the
  28th significant digit.
- **AC2** (R2) — Unlever then relever at the same D/E returns the original beta (property test),
  for both formulas, with and without debt beta.
- **AC3** (R3) — Peer table of five comparables matches the 50-digit reference for mean and
  median.
- **AC4** (R6) — Each invalid input returns its documented error code.

## Interfaces
- Method ids: `coc.cost_of_equity`, `coc.beta_relever`, `coc.beta_peers`, `coc.cost_of_debt`,
  `coc.wacc`.

## Decisions (maintainer, 2026-11-07)
- The tax rate of the tax shield is an explicit input; the engine has no IRAP logic. Analysts
  who include or exclude IRAP do so in the rate they pass, and the rate is echoed in `explain`.
- The specific risk premium is a free input; when it is not zero, a written justification is
  required and echoed in `explain.conventions`.
