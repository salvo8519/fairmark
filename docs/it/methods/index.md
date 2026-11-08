---
source: methods/index.md
source_hash: a3e43346cb45
---
# Metodi

Ogni metodo ha un identificativo stabile, un'implementazione versionata, casi di
riferimento e una pagina metodologica. Il [riferimento dei metodi](../reference/methods.md)
elenca tutti gli input.

Un metodo calcola la conseguenza degli input e delle convenzioni che riceve; la scelta del
metodo, delle convenzioni e degli input, e la verifica del risultato, spettano
all'utilizzatore. Vedi le [avvertenze legali](../disclaimer.md).

<!-- generated:status:start (scripts/gen_reference.py) -->

Stato di ogni metodo, come lo riporta il registro del motore a `fm.methods()`, `fairmark methods`, al server MCP e ai tool di function calling. Definizioni e regola di promozione: [stato dei metodi](https://salvo8519.github.io/fairmark/project/releases/#method-status) (in inglese).

| Stato | Metodi |
| --- | --- |
| Disponibile | `tvm.future_value`, `loans.instalment`, `loans.number_of_instalments`, `loans.amortisation_schedule`, `coc.cost_of_equity`, `coc.beta_relever`, `coc.beta_peers`, `coc.cost_of_debt`, `coc.wacc`, `tv.gordon`, `tv.value_driver`, `tv.h_model`, `tv.exit_multiple`, `tv.limited_horizon`, `tv.normalised_income`, `tv.liquidation`, `tv.compare`, `analysis.goal_seek`, `analysis.sensitivity`, `analysis.scenarios`, `analysis.tornado` |
| Anteprima | `dates.year_fraction`, `rates.convert`, `rates.real_nominal`, `tvm.npv`, `tvm.present_value`, `tvm.xnpv`, `tvm.irr`, `tvm.xirr`, `tvm.mirr`, `tvm.annuity`, `tvm.perpetuity`, `dcf.fcff`, `ratios.profitability`, `ratios.capital_structure`, `ratios.liquidity`, `ratios.working_capital`, `ratios.debt_service`, `ratios.covenants` |

<!-- generated:status:end -->

| Metodo | Che cosa calcola |
| --- | --- |
| [`dates.year_fraction`](dates-year-fraction.md) | Frazione d'anno tra due date secondo una convenzione di calcolo dei giorni ISDA |
| [`rates.convert`](rates-convert.md) | Tasso annuo equivalente con un'altra convenzione di capitalizzazione |
| [`rates.real_nominal`](rates-real-nominal.md) | Tasso reale da un tasso nominale, o viceversa (relazione di Fisher) |
| [`tvm.present_value`](tvm-present-value.md) | Valore attuale di flussi periodici con un unico tasso o un tasso per periodo |
| [`tvm.npv`](tvm-npv.md) | Valore attuale netto di una serie di flussi |
| [`tvm.xnpv`](tvm-xnpv.md) | Valore attuale netto di flussi datati |
| [`tvm.irr`](tvm-irr.md) | Tasso interno di rendimento |
| [`tvm.xirr`](tvm-xirr.md) | Tasso interno di rendimento di flussi datati |
| [`tvm.mirr`](tvm-mirr.md) | Tasso interno di rendimento modificato |
| [`tvm.annuity`](tvm-annuity.md) | Valore attuale e valore futuro di una rendita costante o crescente |
| [`tvm.perpetuity`](tvm-perpetuity.md) | Valore attuale di una rendita perpetua costante o crescente |
| [`tvm.future_value`](tvm-future-value.md) | Valore futuro (montante) di un importo attuale e di pagamenti costanti |
| [`loans.instalment`](loans-instalment.md) | Rata costante di un finanziamento, con balloon e arrotondamento facoltativi |
| [`loans.number_of_instalments`](loans-number-of-instalments.md) | Numero delle rate che rimborsa un finanziamento e ultima rata |
| [`loans.amortisation_schedule`](loans-amortisation-schedule.md) | Piano di ammortamento alla francese, all'italiana o bullet, con preammortamento, balloon e arrotondamento |
| [`coc.cost_of_equity`](coc-cost-of-equity.md) | Costo del capitale proprio con il CAPM e premi espliciti per rischio paese, dimensione e rischio specifico |
| [`coc.beta_relever`](coc-beta-relever.md) | Beta levered da un beta unlevered, o viceversa (Hamada, Harris-Pringle) |
| [`coc.beta_peers`](coc-beta-peers.md) | Beta unlevered medio e mediano delle società comparabili, con relevering alla struttura finanziaria obiettivo |
| [`coc.cost_of_debt`](coc-cost-of-debt.md) | Costo del debito prima e dopo le imposte |
| [`coc.wacc`](coc-wacc.md) | Costo medio ponderato del capitale alla struttura finanziaria obiettivo |
| [`tv.gordon`](tv-gordon.md) | Valore terminale all'orizzonte N a crescita costante, CF_{N+1} / (r − g) |
| [`tv.value_driver`](tv-value-driver.md) | Valore terminale all'orizzonte N con la crescita ottenuta reinvestendo g / RONIC |
| [`tv.h_model`](tv-h-model.md) | Valore terminale all'orizzonte N con crescita in calo lineare (H-model) |
| [`tv.exit_multiple`](tv-exit-multiple.md) | Valore terminale all'orizzonte N come multiplo di una grandezza, con la crescita implicita |
| [`tv.limited_horizon`](tv-limited-horizon.md) | Valore terminale all'orizzonte N di un flusso che dura altri n periodi, più un valore residuo |
| [`tv.normalised_income`](tv-normalised-income.md) | Valore terminale all'orizzonte N come reddito normalizzato capitalizzato |
| [`tv.liquidation`](tv-liquidation.md) | Valore netto di liquidazione all'orizzonte N, ogni voce nella tracciabilità del calcolo |
| [`tv.compare`](tv-compare.md) | Più tecniche di valore terminale su un unico tasso, limite alla crescita e approccio |
| [`dcf.fcff`](dcf-fcff.md) | Value of operations, enterprise value ed equity value dai flussi di cassa operativi, su stage espliciti e un valore terminale |
| [`ratios.profitability`](ratios-profitability.md) | ROE, ROI, ROA, ROS, margini, rotazione del capitale investito e scomposizione del ROE in tre fattori e nella forma con leva finanziaria |
| [`ratios.capital_structure`](ratios-capital-structure.md) | Rapporto tra debito e capitale proprio, PFN su capitale proprio, indice di indipendenza finanziaria, margini e quozienti di struttura |
| [`ratios.liquidity`](ratios-liquidity.md) | Indice di disponibilità, indice di liquidità primaria, cash ratio, capitale circolante netto e margine di tesoreria |
| [`ratios.working_capital`](ratios-working-capital.md) | Giorni medi di incasso, di giacenza e di pagamento e ciclo monetario |
| [`ratios.debt_service`](ratios-debt-service.md) | Copertura degli oneri finanziari, PFN su EBITDA, DSCR per periodo e LLCR |
| [`ratios.covenants`](ratios-covenants.md) | Verifica dei covenant sulle definizioni del contratto di finanziamento: esito, valore verificato e headroom per ogni covenant |
| [`analysis.goal_seek`](analysis-goal-seek.md) | Valore di un input di qualsiasi metodo al quale uno dei suoi output è pari a un valore obiettivo |
| [`analysis.sensitivity`](analysis-sensitivity.md) | Un output di qualsiasi metodo in una tabella di valori degli input a una o a due variabili |
| [`analysis.scenarios`](analysis-scenarios.md) | Output di qualsiasi metodo in scenari nominati, con gli scostamenti rispetto al caso base |
| [`analysis.tornado`](analysis-tornado.md) | Input di qualsiasi metodo ordinati per l'escursione che producono su un output |

## Sviluppi

Le prossime funzionalità, con le loro issue e specifiche, sono nella
[roadmap](https://salvo8519.github.io/fairmark/project/roadmap/) (in inglese), generata dal piano.

Vuoi un metodo? Apri una
[richiesta di metodo](https://github.com/salvo8519/fairmark/issues/new/choose).
