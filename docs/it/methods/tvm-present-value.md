---
source: methods/tvm-present-value.md
source_hash: 9c89e59e8616
---
# Valore attuale di flussi periodici (`tvm.present_value`)

Attualizza i flussi di cassa dei periodi da 1 a $n$ di un piano, con un unico tasso per tutti i
periodi oppure con un tasso diverso per ciascun periodo, e con i flussi collocati alla fine, a
metà o all'inizio del rispettivo periodo. Restituisce il fattore di sconto e il valore attuale
di ciascun flusso, e la loro somma. Serve quando il costo del capitale varia lungo l'orizzonte
del piano, o quando la collocazione temporale dei flussi all'interno di ciascun periodo deve
essere esplicita.

## Formula

Con tassi di periodo $r_1, \ldots, r_n$ (tutti uguali quando è fornito un unico `rate`), il
fattore di sconto a fine periodo del periodo $k$ è il prodotto dei fattori di periodo fino a
$k$:

$$
DF_k^{\text{end}} = \prod_{i=1}^{k} \frac{1}{1 + r_i},
\qquad DF_0^{\text{end}} = 1
$$

Le altre collocazioni partono dal fattore del periodo precedente:

$$
DF_k^{\text{mid}} = \frac{DF_{k-1}^{\text{end}}}{\sqrt{1 + r_k}}
\qquad
DF_k^{\text{beginning}} = DF_{k-1}^{\text{end}}
$$

$$
PV = \sum_{k=1}^{n} CF_k \cdot DF_k
$$

| Collocazione | `timing` | Il flusso $k$ avviene in |
| --- | --- | --- |
| Fine periodo | `end_of_period` | $t = k$ |
| Metà periodo | `mid_period` | $t = k - 0.5$, mezzo periodo al tasso $r_k$ dello stesso periodo |
| Inizio periodo | `beginning_of_period` | $t = k - 1$ |

Con un tasso costante e flussi a fine periodo il risultato coincide con [`tvm.npv`](tvm-npv.md)
degli stessi flussi preceduti da un flusso nullo in $t = 0$.

## Esempio

Tre flussi di 100 con tassi del 5%, 6% e 7% per i periodi da 1 a 3, con ciascuna collocazione:

```python
from decimal import Decimal

import fairmark as fm

flows = [100, 100, 100]
curve = ["0.05", "0.06", "0.07"]

end = fm.tvm.present_value(flows, period_rates=curve, timing="end_of_period")
mid = fm.tvm.present_value(flows, period_rates=curve, timing="mid_period")
beg = fm.tvm.present_value(flows, period_rates=curve, timing="beginning_of_period")

assert round(end.present_value, 18) == Decimal("269.054756446750804007")
assert abs(mid.present_value - Decimal("276.95204711016002295632")) < Decimal("1e-15")
assert round(beg.present_value, 18) == Decimal("285.085354896675651393")
print(end.discount_factors[1])  # 1 / (1.05 × 1.06) = 0.8984725965858041329739442947
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `cash_flows` | Flussi di cassa dei periodi 1, 2, ..., n | obbligatorio |
| `rate` | Un unico tasso di attualizzazione per tutti i periodi, frazione decimale per periodo | fornire esattamente uno tra `rate` e `period_rates` |
| `period_rates` | Un tasso di attualizzazione per periodo (tassi forward di periodo), frazioni decimali; esattamente uno per flusso | fornire esattamente uno tra `rate` e `period_rates` |
| `timing` | `end_of_period`, `mid_period` oppure `beginning_of_period` | obbligatorio |

## Output

| Campo | Significato |
| --- | --- |
| `present_value` | Somma dei valori attuali |
| `discount_factors` | Fattore di sconto applicato a ciascun flusso |
| `present_values` | Valore attuale di ciascun flusso |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Collocazione temporale | fine periodo, metà periodo, inizio periodo | nessuno: deve essere scelta |
| Tassi | un unico tasso per tutti i periodi, oppure un tasso per periodo | nessuno: fornirne esattamente uno |
| Tassi di periodo | forward: $r_k$ si applica solo al periodo $k$ | fissa |
| Metà periodo | mezzo periodo al tasso dello stesso periodo, cioè una divisione per $\sqrt{1 + r_k}$ | fissa |

La collocazione temporale non ha un default perché la prassi varia: i business plan usano
spesso la metà periodo, i contratti la fine periodo. Con un tasso costante, l'attualizzazione a
metà periodo è il consueto $(1 + r)^{-(k - 0.5)}$.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Sono forniti entrambi o nessuno tra `rate` e `period_rates`, `period_rates` non contiene esattamente un tasso per flusso, oppure un campo è mancante o sconosciuto |
| `FM-E002` | `cash_flows` è vuoto |
| `FM-E003` | Un tasso di periodo è pari o inferiore a −100% |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

I fattori a fine periodo e a inizio periodo usano solo divisioni; rispetto a un calcolo
indipendente a 60 cifre, concordano entro circa $10^{-24}$ in termini relativi o meglio. I
fattori a metà periodo usano la radice quadrata decimale, che concorda entro circa $10^{-18}$
in termini relativi o meglio su input tipici. I casi di riferimento sono verificati entro
$10^{-18}$ per le collocazioni a fine e inizio periodo ed entro $10^{-19}$ per la metà periodo.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/tvm.present_value/`, da un calcolo
  indipendente a 50 cifre (`scripts/reference_time.py`).
- Con un tasso costante e flussi a fine periodo, concorda con la funzione `NPV` dei fogli di
  calcolo (Excel, LibreOffice), che attualizza il primo valore di un periodo, e con
  `numpy_financial.npv` applicato agli stessi flussi preceduti da un flusso nullo.
