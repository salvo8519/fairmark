---
source: methods/dcf-fcff.md
source_hash: 22cec8dfd864
---
# DCF, flussi di cassa operativi (`dcf.fcff`)

Valuta l'attività operativa attualizzando i flussi di cassa operativi disponibili (FCFF) al
costo medio ponderato del capitale (WACC), poi passa dall'enterprise value all'equity value.

## Formula

$$
EV = \sum_{t=1}^{n} \frac{FCFF_t}{(1+WACC)^{\tau_t}} + \frac{TV}{(1+WACC)^{n}}
\qquad
TV = \frac{FCFF_n \,(1+g)}{WACC - g}
$$

$$
Equity = EV - \text{PFN} + \text{attività non operative}
$$

dove $\tau_t = t$ (fine periodo) oppure $\tau_t = t - 0.5$ (metà periodo).

## Esempio

```python
import fairmark as fm

r = fm.dcf.fcff(
    [250, 270, 290, 300, 310],
    discount_rate="0.085",
    terminal_growth="0.015",
    timing="mid_period",
    net_debt=-50,              # cassa netta positiva
    non_operating_assets=30,
)
print(r.enterprise_value)       # 4145.01655907...
print(r.terminal_value_weight)  # quota dell'EV che deriva dal valore terminale
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `free_cash_flows` | FCFF degli anni espliciti 1..n | obbligatorio |
| `discount_rate` | WACC come frazione decimale | obbligatorio |
| `terminal_growth` | Crescita perpetua dopo l'anno n; deve essere < WACC | obbligatorio |
| `timing` | `end_of_period` oppure `mid_period` | `end_of_period` |
| `net_debt` | Posizione finanziaria netta: debiti meno cassa; negativa se cassa netta | `0` |
| `non_operating_assets` | Attività non operative aggiunte all'equity | `0` |

## Convenzioni (esplicite, riportate in `explain.conventions`)

- Il valore terminale è collocato alla fine dell'anno n e attualizzato in `t = n`, anche con la
  convenzione di metà periodo. La prassi varia; è previsto un parametro per modificarlo.
- `terminal_value_weight` indica la quota dell'EV che deriva dal valore terminale: un peso
  elevato segnala che la valutazione dipende soprattutto dalle ipotesi di lungo periodo.

## In programma (M2)

Costruzione dei FCFF dal bilancio d'esercizio, WACC/CAPM con premio dimensionale e rischio
specifico, valore terminale con multiplo di uscita, griglie di sensitività.
