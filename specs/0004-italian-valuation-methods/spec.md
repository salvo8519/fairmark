# SPEC-0004 — Asset-based, income-based, mixed and EVA methods (Italian practice)

| | |
| --- | --- |
| Status | Draft — proposed for approval |
| Milestone | M1 |
| Owner | @salvo8519 |
| Formula basis | Formulas commonly adopted in Italian practice (asset-based, income-based, mixed UEC, EVA); cross-checked against published worked examples and practitioner models reviewed by the maintainer |
| Related | ADR 0008; SPEC-0002, SPEC-0003; profiles per ADR 0001 (practice as explicit profiles) |

## Problem
Italian practice, from court-appointed valuations to transactions between small companies,
relies on asset-based, income-based and mixed methods as much as on DCF. Their formulas are
simple but the inputs (adjustments, normalised income, rates, goodwill horizon, latent taxes)
are where judgement lies, and where spreadsheets hide errors.

## Scope
**In scope:** simple and complex asset-based method; income-based method (perpetual and finite
horizon, with optional growth); normalised income from historical and forecast incomes with
weights; mixed method with separate goodwill estimate (UEC); mixed average method; EVA method.
**Out of scope:** market multiples (separate spec); valuation of specific intangibles
(royalty relief and similar, separate spec); real estate appraisal methods (separate spec).

## Requirements (EARS)
- **R1** — WHEN given book equity and a list of labelled adjustments (revaluations and
  write-downs) and a tax rate for latent taxes, THE ENGINE SHALL return the adjusted equity
  K = book equity + Σ adjustments − tax × Σ taxable adjustments, with each line in `explain`
  and the latent-tax treatment (full, reduced rate, none) as an explicit input.
- **R2** — WHEN additionally given separately valued intangibles not recognised in the balance
  sheet, THE ENGINE SHALL return the complex asset-based value K + Σ intangibles (net of tax
  when declared).
- **R3** — WHEN given a series of incomes with weights, THE ENGINE SHALL return the normalised
  income as their weighted mean and report the weights; equal weights SHALL be an explicit choice.
- **R4** — WHEN given a normalised income R and a rate i, THE ENGINE SHALL return the income-based
  value R / i (perpetual), R × a(n, i) (finite horizon) or R / (i − g) (growing), as selected.
- **R5** — WHEN given K, R, the normal return rate i on K, the discount rate i′ for excess income
  and a horizon n, THE ENGINE SHALL return the mixed value with separate goodwill estimate
  W = K + a(n, i′) × (R − i × K), reporting goodwill (positive) or badwill (negative); a
  perpetual excess-income option SHALL be available.
- **R6** — WHEN given K and an income-based value, THE ENGINE SHALL return the mixed average
  value W = w × K + (1 − w) × R / i with explicit weight w.
- **R7** — WHEN given invested capital at the start, NOPAT per period and WACC, THE ENGINE SHALL
  compute EVA_t = NOPAT_t − WACC × IC_{t−1}, the market value added as their present value plus a
  terminal value, and the value W = IC_0 + MVA (minus net debt for equity value).
- **R8** — IF a rate is not positive, n < 1, weights do not sum to 1, or g ≥ i, THEN THE ENGINE
  SHALL return a typed error.

## Conventions and parameters
| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Latent taxes on adjustments | full rate, reduced rate, none | required | PIV; practice varies |
| Income horizon | perpetual, finite n, growing | required | Guatri, Bini |
| Goodwill horizon n and rate i′ | inputs | required | UEC method; practice |
| Normal return rate i | input | required | PIV |
| Normalised income weights | explicit list | required | practice |

## Acceptance criteria
- **AC1** (R1, R2) — Asset-based example with five adjustments matches the reference.
- **AC2** (R3, R4) — Income-based values in the three variants match the reference.
- **AC3** (R5) — UEC example with goodwill and one with badwill match the reference; the
  perpetual variant equals K + (R − iK) / i′.
- **AC4** (R6, R7) — Mixed average and EVA examples match the reference; EVA value equals the
  DCF value under consistent assumptions (property test, Koller).
- **AC5** (R8) — Each invalid input returns its documented error code.

## Interfaces
- Method ids: `asset.adjusted_equity`, `asset.complex`, `income.normalised`, `income.value`,
  `mixed.uec`, `mixed.average`, `eva.value`.

## Open questions
- [ ] Practitioner cases (anonymised) from the maintainer to use as golden cases next to
      textbook examples.
- [ ] Latent taxes: is "reduced rate" a percentage of the full rate or a rate of its own?
