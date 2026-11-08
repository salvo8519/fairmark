# Methods

Each method has a stable id, a versioned implementation, golden test cases and a methodology
page. The [methods reference](../reference/methods.md) lists every input.

A method computes the consequence of the inputs and conventions it receives; choosing the
method, the conventions and the inputs, and verifying the result, belong to the user. See the
[disclaimer](../disclaimer.md).

<!-- generated:status:start (scripts/gen_reference.py) -->

Status of each method, as the engine registry reports it to `fm.methods()`, `fairmark methods`, the MCP server and function-calling tools. Definitions and the promotion rule: [method status](../project/releases.md#method-status).

| Status | Methods |
| --- | --- |
| Available | `dates.year_fraction`, `rates.convert`, `rates.real_nominal`, `tvm.npv`, `tvm.present_value`, `tvm.xnpv`, `tvm.irr`, `tvm.xirr`, `tvm.mirr`, `tvm.annuity`, `tvm.perpetuity`, `tvm.future_value`, `loans.instalment`, `loans.number_of_instalments`, `loans.amortisation_schedule`, `coc.cost_of_equity`, `coc.beta_relever`, `coc.beta_peers`, `coc.cost_of_debt`, `coc.wacc`, `tv.gordon`, `tv.value_driver`, `tv.h_model`, `tv.exit_multiple`, `tv.limited_horizon`, `tv.normalised_income`, `tv.liquidation`, `tv.compare`, `analysis.goal_seek`, `analysis.sensitivity`, `analysis.scenarios`, `analysis.tornado` |
| Preview | `dates.schedule`, `series.project`, `dcf.fcff`, `dcf.fcfe`, `dcf.free_cash_flows`, `statements.reorganise`, `ratios.profitability`, `ratios.capital_structure`, `ratios.liquidity`, `ratios.working_capital`, `ratios.debt_service`, `ratios.covenants` |

<!-- generated:status:end -->

| Method | What it computes |
| --- | --- |
| [`dates.year_fraction`](dates-year-fraction.md) | Year fraction between two dates under an ISDA day count |
| [`dates.schedule`](dates-schedule.md) | Start and end dates of contiguous accrual periods [start, end), with year fractions |
| [`series.project`](series-project.md) | Values of periods 1 to n grown from a base value by a constant, linear or per-period rule, dated on a schedule |
| [`rates.convert`](rates-convert.md) | Equivalent annual rate under another compounding convention |
| [`rates.real_nominal`](rates-real-nominal.md) | Real rate from a nominal rate, or the reverse (Fisher relation) |
| [`tvm.present_value`](tvm-present-value.md) | Present value of periodic flows with one rate or one rate per period |
| [`tvm.npv`](tvm-npv.md) | Net present value of a cash flow series |
| [`tvm.xnpv`](tvm-xnpv.md) | Net present value of dated cash flows |
| [`tvm.irr`](tvm-irr.md) | Internal rate of return |
| [`tvm.xirr`](tvm-xirr.md) | Internal rate of return of dated cash flows |
| [`tvm.mirr`](tvm-mirr.md) | Modified internal rate of return |
| [`tvm.annuity`](tvm-annuity.md) | Present and future value of a level or growing annuity |
| [`tvm.perpetuity`](tvm-perpetuity.md) | Present value of a level or growing perpetuity |
| [`tvm.future_value`](tvm-future-value.md) | Future value of a present amount and level payments |
| [`loans.instalment`](loans-instalment.md) | Constant instalment of a loan, with an optional balloon amount and rounding |
| [`loans.number_of_instalments`](loans-number-of-instalments.md) | Number of instalments that repays a loan and the final instalment |
| [`loans.amortisation_schedule`](loans-amortisation-schedule.md) | Amortisation schedule: French, constant principal or bullet, with grace period, balloon and rounding |
| [`coc.cost_of_equity`](coc-cost-of-equity.md) | Cost of equity by CAPM with explicit country, size and specific premia |
| [`coc.beta_relever`](coc-beta-relever.md) | Levered beta from an unlevered beta, or the reverse (Hamada, Harris-Pringle) |
| [`coc.beta_peers`](coc-beta-peers.md) | Mean and median unlevered beta of comparables, relevered to a target structure |
| [`coc.cost_of_debt`](coc-cost-of-debt.md) | Pre-tax and after-tax cost of debt |
| [`coc.wacc`](coc-wacc.md) | Weighted average cost of capital at a target capital structure |
| [`tv.gordon`](tv-gordon.md) | Terminal value at N with stable growth, CF_{N+1} / (r − g) |
| [`tv.value_driver`](tv-value-driver.md) | Terminal value at N with growth earned by reinvesting g / RONIC |
| [`tv.h_model`](tv-h-model.md) | Terminal value at N with growth declining linearly (H-model) |
| [`tv.exit_multiple`](tv-exit-multiple.md) | Terminal value at N as a multiple of a metric, with the implied growth |
| [`tv.limited_horizon`](tv-limited-horizon.md) | Terminal value at N of a flow lasting n more periods, plus a residual value |
| [`tv.normalised_income`](tv-normalised-income.md) | Terminal value at N as a capitalised normalised income |
| [`tv.liquidation`](tv-liquidation.md) | Net liquidation value at N, every line in the audit trail |
| [`tv.compare`](tv-compare.md) | Several terminal value techniques on one rate, growth cap and basis |
| [`dcf.fcff`](dcf-fcff.md) | Value of operations, enterprise value and equity value from unlevered free cash flows over explicit stages and a terminal value |
| [`dcf.fcfe`](dcf-fcfe.md) | Equity value directly from free cash flows to equity at the cost of equity over explicit stages and a terminal value, without a net financial position |
| [`dcf.free_cash_flows`](dcf-free-cash-flows.md) | FCFF and FCFE per period from EBIT, taxes, capital expenditure and working capital |
| [`statements.reorganise`](statements-reorganise.md) | Statutory aggregates or reclassified totals reorganised into invested capital, non-operating assets, equity, the net financial position and debt-like items, with current and non-current amounts, and the income statement into value added, EBITDA, operating profit, investing result and profit before financing and taxes (IFRS 18 subtotals); best effort, checked exactly |
| [`ratios.profitability`](ratios-profitability.md) | ROE, ROI, ROA, ROS, margins, capital turnover and the ROE decomposition in three factors and in the leverage form |
| [`ratios.capital_structure`](ratios-capital-structure.md) | Debt to equity, net financial position to equity, equity ratio, structure margins and fixed-asset coverage |
| [`ratios.liquidity`](ratios-liquidity.md) | Current, quick and cash ratios, net working capital and treasury margin |
| [`ratios.working_capital`](ratios-working-capital.md) | Days sales, inventory and payable outstanding and the cash conversion cycle |
| [`ratios.debt_service`](ratios-debt-service.md) | Interest cover, net financial position to EBITDA, DSCR per period and LLCR |
| [`ratios.covenants`](ratios-covenants.md) | Covenant tests on the definitions of the loan agreement: outcome, tested value and headroom per covenant |
| [`analysis.goal_seek`](analysis-goal-seek.md) | Value of one input of any method at which one of its outputs equals a target |
| [`analysis.sensitivity`](analysis-sensitivity.md) | One output of any method across a one-way or two-way table of input values |
| [`analysis.scenarios`](analysis-scenarios.md) | Outputs of any method under named cases, with deltas against the base case |
| [`analysis.tornado`](analysis-tornado.md) | Inputs of any method ranked by the swing they cause in one output |

## Next

What comes next, with its issues and specifications, is in the
[roadmap](../project/roadmap.md), generated from the plan.

Want a method? Open a [method request](https://github.com/salvo8519/fairmark/issues/new/choose).
