---
source: methods/index.md
source_hash: 29504fb61a41
---
# Metodi

Ogni metodo ha un identificativo stabile, un'implementazione versionata, casi di
riferimento e una pagina metodologica. Il [riferimento dei metodi](../reference/methods.md)
elenca tutti gli input.

Un metodo calcola la conseguenza degli input e delle convenzioni che riceve; la scelta del
metodo, delle convenzioni e degli input, e la verifica del risultato, spettano
all'utilizzatore. Vedi le [avvertenze legali](../disclaimer.md).

| Metodo | Stato | Che cosa calcola |
| --- | --- | --- |
| [`dates.year_fraction`](dates-year-fraction.md) | Disponibile | Frazione d'anno tra due date secondo una convenzione di calcolo dei giorni ISDA |
| [`rates.convert`](rates-convert.md) | Disponibile | Tasso annuo equivalente con un'altra convenzione di capitalizzazione |
| [`rates.real_nominal`](rates-real-nominal.md) | Disponibile | Tasso reale da un tasso nominale, o viceversa (relazione di Fisher) |
| [`tvm.present_value`](tvm-present-value.md) | Disponibile | Valore attuale di flussi periodici con un unico tasso o un tasso per periodo |
| [`tvm.npv`](tvm-npv.md) | Disponibile | Valore attuale netto di una serie di flussi |
| [`tvm.xnpv`](tvm-xnpv.md) | Disponibile | Valore attuale netto di flussi datati |
| [`tvm.irr`](tvm-irr.md) | Disponibile | Tasso interno di rendimento |
| [`tvm.xirr`](tvm-xirr.md) | Disponibile | Tasso interno di rendimento di flussi datati |
| [`tvm.mirr`](tvm-mirr.md) | Disponibile | Tasso interno di rendimento modificato |
| [`tvm.annuity`](tvm-annuity.md) | Disponibile | Valore attuale e valore futuro di una rendita costante o crescente |
| [`tvm.perpetuity`](tvm-perpetuity.md) | Disponibile | Valore attuale di una rendita perpetua costante o crescente |
| [`tvm.future_value`](tvm-future-value.md) | Disponibile | Valore futuro (montante) di un importo attuale e di pagamenti costanti |
| [`loans.instalment`](loans-instalment.md) | Disponibile | Rata costante di un finanziamento, con balloon e arrotondamento facoltativi |
| [`loans.number_of_instalments`](loans-number-of-instalments.md) | Disponibile | Numero delle rate che rimborsa un finanziamento e ultima rata |
| [`loans.amortisation_schedule`](loans-amortisation-schedule.md) | Disponibile | Piano di ammortamento alla francese, all'italiana o bullet, con preammortamento, balloon e arrotondamento |
| [`coc.cost_of_equity`](coc-cost-of-equity.md) | Disponibile | Costo del capitale proprio con il CAPM e premi espliciti per rischio paese, dimensione e rischio specifico |
| [`coc.beta_relever`](coc-beta-relever.md) | Disponibile | Beta levered da un beta unlevered, o viceversa (Hamada, Harris-Pringle) |
| [`coc.beta_peers`](coc-beta-peers.md) | Disponibile | Beta unlevered medio e mediano delle società comparabili, con relevering alla struttura finanziaria obiettivo |
| [`coc.cost_of_debt`](coc-cost-of-debt.md) | Disponibile | Costo del debito prima e dopo le imposte |
| [`coc.wacc`](coc-wacc.md) | Disponibile | Costo medio ponderato del capitale alla struttura finanziaria obiettivo |
| [`tv.gordon`](tv-gordon.md) | Disponibile | Valore terminale all'orizzonte N a crescita costante, CF_{N+1} / (r − g) |
| [`tv.value_driver`](tv-value-driver.md) | Disponibile | Valore terminale all'orizzonte N con la crescita ottenuta reinvestendo g / RONIC |
| [`tv.h_model`](tv-h-model.md) | Disponibile | Valore terminale all'orizzonte N con crescita in calo lineare (H-model) |
| [`tv.exit_multiple`](tv-exit-multiple.md) | Disponibile | Valore terminale all'orizzonte N come multiplo di una grandezza, con la crescita implicita |
| [`tv.limited_horizon`](tv-limited-horizon.md) | Disponibile | Valore terminale all'orizzonte N di un flusso che dura altri n periodi, più un valore residuo |
| [`tv.normalised_income`](tv-normalised-income.md) | Disponibile | Valore terminale all'orizzonte N come reddito normalizzato capitalizzato |
| [`tv.liquidation`](tv-liquidation.md) | Disponibile | Valore netto di liquidazione all'orizzonte N, ogni voce nella tracciabilità del calcolo |
| [`tv.compare`](tv-compare.md) | Disponibile | Più tecniche di valore terminale su un unico tasso, limite alla crescita e approccio |
| [`dcf.fcff`](dcf-fcff.md) | Anteprima | Enterprise value ed equity value dai flussi di cassa operativi |
| [`ratios.profitability`](ratios-profitability.md) | Anteprima | ROE, ROI, ROA, ROS, margini, rotazione del capitale investito e scomposizione del ROE in tre fattori e nella forma con leva finanziaria |
| [`ratios.capital_structure`](ratios-capital-structure.md) | Anteprima | Rapporto tra debito e capitale proprio, PFN su capitale proprio, indice di indipendenza finanziaria, margini e quozienti di struttura |
| [`ratios.liquidity`](ratios-liquidity.md) | Anteprima | Indice di disponibilità, indice di liquidità primaria, cash ratio, capitale circolante netto e margine di tesoreria |
| [`ratios.working_capital`](ratios-working-capital.md) | Anteprima | Giorni medi di incasso, di giacenza e di pagamento e ciclo monetario |
| [`ratios.debt_service`](ratios-debt-service.md) | Anteprima | Copertura degli oneri finanziari, PFN su EBITDA, DSCR per periodo e LLCR |
| [`ratios.covenants`](ratios-covenants.md) | Anteprima | Verifica dei covenant sulle definizioni del contratto di finanziamento: esito, valore verificato e headroom per ogni covenant |
| [`analysis.goal_seek`](analysis-goal-seek.md) | Disponibile | Valore di un input di qualsiasi metodo al quale uno dei suoi output è pari a un valore obiettivo |
| [`analysis.sensitivity`](analysis-sensitivity.md) | Disponibile | Un output di qualsiasi metodo in una tabella di valori degli input a una o a due variabili |
| [`analysis.scenarios`](analysis-scenarios.md) | Disponibile | Output di qualsiasi metodo in scenari nominati, con gli scostamenti rispetto al caso base |
| [`analysis.tornado`](analysis-tornado.md) | Disponibile | Input di qualsiasi metodo ordinati per l'escursione che producono su un output |

## Sviluppi

Le prossime funzionalità, con le loro issue e specifiche, sono nella
[roadmap](https://salvo8519.github.io/fairmark/project/roadmap/) (in inglese), generata dal piano.

Vuoi un metodo? Apri una
[richiesta di metodo](https://github.com/salvo8519/fairmark/issues/new/choose).
