# SPEC-0003 — Plan

## Design
- One module, `coc.rs`, with five methods. All formulas are rational, so results are exact to
  28 significant digits and golden cases use closed forms written out in the file, checkable by
  hand (tolerance zero where the value terminates).
- `BetaFormula` carries the leverage factor ((1 − t) D/E for Hamada, D/E for Harris-Pringle) and
  both directions, so relevering and unlevering share one definition and round-trip exactly.
- `coc.cost_of_equity` accepts either a levered beta or an unlevered beta to relever, so an
  agent can go from comparables to the cost of equity in two calls (`coc.beta_peers`, then
  `coc.cost_of_equity`) without computing anything itself.
- Mutually exclusive inputs (levered beta or relever; pre-tax rate or rate plus spread; D/E or
  D/V) are validated explicitly: both or neither is `FM-E001`.
- Defaults (premia 0, country exposure 1, debt beta 0) are documented in the schema and
  reported as "(default)" in `explain.conventions`.

## Schema changes
Additive: five new method schemas.

## Risks
- Analysts may expect the WACC to accept market-value weights computed from prices: out of
  scope; weights are target weights given as D/E or D/V.
