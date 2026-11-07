# Methods

Each method has a stable id, a versioned implementation, golden test cases and a methodology
page. The [methods reference](../reference/methods.md) lists every input.

| Method | Status | What it computes |
| --- | --- | --- |
| [`tvm.npv`](tvm-npv.md) | available | Net present value of a cash flow series |
| [`tvm.irr`](tvm-irr.md) | available | Internal rate of return |
| [`dcf.fcff`](dcf-fcff.md) | preview | Enterprise and equity value from unlevered free cash flows |

## Coming next

- **M2 — complete DCF**: FCFF/FCFE from statutory financial statements, WACC and CAPM,
  exit-multiple terminal value, sensitivity grids.
- **M4 — European practice**: asset-based, income-based and mixed (excess earnings) methods,
  EVA, market multiples.
- **Real estate**: property DCF, direct capitalisation, debt and equity waterfalls.

Want a method? Open a [method request](https://github.com/salvo8519/fairmark/issues/new/choose).
