# Methods

Each method has a stable id, a versioned implementation, golden test cases and a methodology
page. The [methods reference](../reference/methods.md) lists every input.

A method computes the consequence of the inputs and conventions it receives; choosing the
method, the conventions and the inputs, and verifying the result, belong to the user. See the
[disclaimer](../disclaimer.md).

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
| [`tvm.future_value`](tvm-future-value.md) | available | Future value of a present amount and level payments |
| [`loans.instalment`](loans-instalment.md) | available | Constant instalment of a loan, with an optional balloon amount and rounding |
| [`loans.number_of_instalments`](loans-number-of-instalments.md) | available | Number of instalments that repays a loan and the final instalment |
| [`loans.amortisation_schedule`](loans-amortisation-schedule.md) | available | Amortisation schedule: French, constant principal or bullet, with grace period, balloon and rounding |
| [`coc.cost_of_equity`](coc-cost-of-equity.md) | available | Cost of equity by CAPM with explicit country, size and specific premia |
| [`coc.beta_relever`](coc-beta-relever.md) | available | Levered beta from an unlevered beta, or the reverse (Hamada, Harris-Pringle) |
| [`coc.beta_peers`](coc-beta-peers.md) | available | Mean and median unlevered beta of comparables, relevered to a target structure |
| [`coc.cost_of_debt`](coc-cost-of-debt.md) | available | Pre-tax and after-tax cost of debt |
| [`coc.wacc`](coc-wacc.md) | available | Weighted average cost of capital at a target capital structure |
| [`tv.gordon`](tv-gordon.md) | available | Terminal value at N with stable growth, CF_{N+1} / (r − g) |
| [`tv.value_driver`](tv-value-driver.md) | available | Terminal value at N with growth earned by reinvesting g / RONIC |
| [`tv.h_model`](tv-h-model.md) | available | Terminal value at N with growth declining linearly (H-model) |
| [`tv.exit_multiple`](tv-exit-multiple.md) | available | Terminal value at N as a multiple of a metric, with the implied growth |
| [`tv.limited_horizon`](tv-limited-horizon.md) | available | Terminal value at N of a flow lasting n more periods, plus a residual value |
| [`tv.normalised_income`](tv-normalised-income.md) | available | Terminal value at N as a capitalised normalised income |
| [`tv.liquidation`](tv-liquidation.md) | available | Net liquidation value at N, every line in the audit trail |
| [`tv.compare`](tv-compare.md) | available | Several terminal value techniques on one rate, growth cap and basis |
| [`dcf.fcff`](dcf-fcff.md) | preview | Enterprise and equity value from unlevered free cash flows |
| [`ratios.profitability`](ratios-profitability.md) | preview | ROE, ROI, ROA, ROS, margins, capital turnover and the ROE decomposition in three factors and in the leverage form |
| [`ratios.capital_structure`](ratios-capital-structure.md) | preview | Debt to equity, net financial position to equity, equity ratio, structure margins and fixed-asset coverage |
| [`ratios.liquidity`](ratios-liquidity.md) | preview | Current, quick and cash ratios, net working capital and treasury margin |
| [`ratios.working_capital`](ratios-working-capital.md) | preview | Days sales, inventory and payable outstanding and the cash conversion cycle |
| [`ratios.debt_service`](ratios-debt-service.md) | preview | Interest cover, net financial position to EBITDA, DSCR per period and LLCR |
| [`ratios.covenants`](ratios-covenants.md) | preview | Covenant tests on the definitions of the loan agreement: outcome, tested value and headroom per covenant |
| [`analysis.goal_seek`](analysis-goal-seek.md) | available | Value of one input of any method at which one of its outputs equals a target |
| [`analysis.sensitivity`](analysis-sensitivity.md) | available | One output of any method across a one-way or two-way table of input values |
| [`analysis.scenarios`](analysis-scenarios.md) | available | Outputs of any method under named cases, with deltas against the base case |
| [`analysis.tornado`](analysis-tornado.md) | available | Inputs of any method ranked by the swing they cause in one output |

## Next

What comes next, with its issues and specifications, is in the
[roadmap](../project/roadmap.md), generated from the plan.

Want a method? Open a [method request](https://github.com/salvo8519/fairmark/issues/new/choose).
