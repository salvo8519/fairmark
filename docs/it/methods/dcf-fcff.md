---
source: methods/dcf-fcff.md
source_hash: b069f01e8b3b
---
# DCF, flussi di cassa operativi (`dcf.fcff`)

Valuta l'attività operativa attualizzando i flussi di cassa operativi disponibili (FCFF) al
costo medio ponderato del capitale (WACC), poi passa dall'enterprise value all'equity value.

Versione del metodo: 2. La versione 2 calcola i fattori di sconto per tassi negativi dal
reciproco di $1 + r$; solo le ultime cifre dei risultati con tassi negativi differiscono dalla
versione 1.

## Formula

$$
EV = \sum_{t=1}^{n} \frac{FCFF_t}{(1+WACC)^{\tau_t}} + \frac{TV}{(1+WACC)^{n}}
\qquad
TV = \frac{FCFF_n \,(1+g)}{WACC - g}
$$

$$
Equity = EV - \text{PFN} + \text{attività non operative}
$$

dove $\tau_t = t$ (fine periodo) oppure $\tau_t = t - 0.5$ (metà periodo). Il valore terminale
è calcolato con il modello di crescita di Gordon, come in [`tvm.perpetuity`](tvm-perpetuity.md).

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
| `discount_rate` | WACC annuo, frazione decimale | obbligatorio |
| `terminal_growth` | Crescita perpetua dopo l'anno n, frazione decimale; deve essere inferiore al WACC | obbligatorio |
| `timing` | `end_of_period` oppure `mid_period` | `end_of_period` |
| `net_debt` | Posizione finanziaria netta: debiti meno cassa; negativa se cassa netta | `0` |
| `non_operating_assets` | Attività non operative (surplus asset) aggiunte all'equity | `0` |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Collocazione temporale dei flussi espliciti | fine periodo, metà periodo | fine periodo |
| Valore terminale | collocato alla fine dell'anno n e attualizzato in $t = n$, anche con la convenzione di metà periodo | fissa |

La prassi varia sulla collocazione del valore terminale; è previsto un parametro per
modificarla. `terminal_value_weight` indica la quota dell'EV che deriva dal valore terminale:
un peso elevato segnala che la valutazione dipende soprattutto dalle ipotesi di lungo periodo.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Un campo è mancante o sconosciuto |
| `FM-E002` | `free_cash_flows` è vuoto |
| `FM-E003` | Il tasso di attualizzazione è pari o inferiore a −100% |
| `FM-E004` | Il tasso di crescita di lungo periodo è pari o superiore al tasso di attualizzazione |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

L'attualizzazione a fine periodo usa potenze intere esatte; rispetto a un calcolo indipendente
a 60 cifre, queste operazioni concordano entro circa $10^{-24}$ in termini relativi o meglio.
L'attualizzazione a metà periodo usa potenze frazionarie, che concordano entro circa
$10^{-18}$ in termini relativi o meglio su input tipici. I casi di riferimento sono verificati
entro $10^{-9}$.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/dcf.fcff/`, da un'implementazione
  decimale indipendente (`scripts/reference_values.py`).

## In programma (M2)

Costruzione dei FCFF dal bilancio d'esercizio, WACC/CAPM con premio dimensionale e rischio
specifico, valore terminale con multiplo di uscita, griglie di sensitività.
