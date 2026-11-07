# Methods

Each method has a stable id, a versioned implementation, golden test cases and a methodology
page. The [methods reference](../reference/methods.md) lists every input.

| Method | Status | What it computes |
| --- | --- | --- |
| [`dates.year_fraction`](dates-year-fraction.md) | available | Year fraction between two dates under an ISDA day count |
| [`rates.convert`](rates-convert.md) | available | Equivalent annual rate under another compounding convention |
| [`rates.real_nominal`](rates-real-nominal.md) | available | Real rate from a nominal rate, or the reverse (Fisher relation) |
| [`tvm.present_value`](tvm-present-value.md) | available | Present value of periodic flows with one rate or one rate per period |
| [`tvm.npv`](tvm-npv.md) | available | Net present value of a cash flow series |
| [`tvm.xnpv`](tvm-xnpv.md) | available | Net present value of dated cash flows |
| [`tvm.irr`](tvm-irr.md) | available | Internal rate of return |
| [`tvm.xirr`](tvm-xirr.md) | available | Internal rate of return of dated cash flows |
| [`tvm.mirr`](tvm-mirr.md) | available | Modified internal rate of return |
| [`tvm.annuity`](tvm-annuity.md) | available | Present and future value of a level or growing annuity |
| [`tvm.perpetuity`](tvm-perpetuity.md) | available | Present value of a level or growing perpetuity |
| [`coc.cost_of_equity`](coc-cost-of-equity.md) | available | Cost of equity by CAPM with explicit country, size and specific premia |
| [`coc.beta_relever`](coc-beta-relever.md) | available | Levered beta from an unlevered beta, or the reverse (Hamada, Harris-Pringle) |
| [`coc.beta_peers`](coc-beta-peers.md) | available | Mean and median unlevered beta of comparables, relevered to a target structure |
| [`coc.cost_of_debt`](coc-cost-of-debt.md) | available | Pre-tax and after-tax cost of debt |
| [`coc.wacc`](coc-wacc.md) | available | Weighted average cost of capital at a target capital structure |
| [`dcf.fcff`](dcf-fcff.md) | preview | Enterprise and equity value from unlevered free cash flows |

## Coming next

- **M2 — complete DCF**: FCFF/FCFE from statutory financial statements, exit-multiple
  terminal value, sensitivity grids.
- **M4 — European practice**: asset-based, income-based and mixed (excess earnings) methods,
  EVA, market multiples.
- **Real estate**: property DCF, direct capitalisation, debt and equity waterfalls.

Want a method? Open a [method request](https://github.com/salvo8519/fairmark/issues/new/choose).
