---
source: methods/tv-h-model.md
source_hash: 6e59c91326ae
---
# Valore terminale con l'H-model (`tv.h_model`)

Restituisce il valore terminale all'orizzonte N quando in N la crescita è ancora sopra il suo
livello stabile e cala linearmente da una crescita di breve periodo $g_s$ a una crescita di
lungo periodo $g_l$ in 2H periodi, per poi restare a $g_l$ per sempre. L'H-model (Fuller e
Hsia) è un'approssimazione in forma chiusa di questo andamento: il valore a crescita stabile
più il valore della crescita aggiuntiva durante il declino. Si usa quando in N è già in corso
una transizione.

## Formula

Per $g_l < r$ e $H \ge 0$:

$$
TV_N = \frac{CF_N\,(1 + g_l)}{r - g_l} + \frac{CF_N\,H\,(g_s - g_l)}{r - g_l}
= CF_N\,\frac{(1 + g_l) + H\,(g_s - g_l)}{r - g_l}
$$

Con $g_s = g_l$, oppure $H = 0$, il secondo termine è nullo e il risultato coincide con
[`tv.gordon`](tv-gordon.md) sull'ultimo flusso. La formula è un'approssimazione, non il valore
esatto di una crescita in calo lineare; il metodo lo dichiara in `explain.conventions`.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

# CF_N 100, crescita dal 10% al 3% in 2H = 10 periodi, tasso 9%
r = fm.tv.h_model(100, "0.10", "0.03", 5, "0.09")
assert r.terminal_value == Decimal("2300")
assert round(r.extra_growth_value, 10) == Decimal("583.3333333333")

# Crescite uguali: l'H-model è il modello di Gordon sull'ultimo flusso
same = fm.tv.h_model(100, "0.02", "0.02", 4, "0.09")
gordon = fm.tv.gordon("0.09", "0.02", last_flow=100)
assert same.terminal_value == gordon.terminal_value
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `last_flow` | Flusso del periodo N, in unità monetarie | obbligatorio |
| `short_term_growth` | $g_s$, crescita all'inizio del declino, frazione decimale per periodo, maggiore di −1 | obbligatorio |
| `long_term_growth` | $g_l$, crescita stabile, frazione decimale per periodo, maggiore di −1 e inferiore a `rate` | obbligatorio |
| `half_life` | H, in periodi, zero o positivo, anche frazionario; il declino dura 2H periodi | obbligatorio |
| `rate` | Tasso di attualizzazione della fase terminale, frazione decimale per periodo | obbligatorio |
| `growth_cap` | Limite alla crescita di lungo periodo, frazione decimale per periodo | nessuno, riportato come assente |

## Output

| Campo | Significato |
| --- | --- |
| `terminal_value` | Valore terminale in N, non attualizzato |
| `stable_growth_value` | $CF_N (1 + g_l) / (r - g_l)$ |
| `extra_growth_value` | $CF_N H (g_s - g_l) / (r - g_l)$ |
| `growth_cap` | La crescita di lungo periodo confrontata con il limite; `null` senza limite |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Andamento della crescita | calo lineare in 2H periodi, poi $g_l$ (approssimazione) | fisso |
| Crescita confrontata con il limite | la crescita di lungo periodo $g_l$; $g_s$ è transitoria | fissa |
| Tasso della fase terminale | fornito | nessuno: va fornito |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Una crescita è pari o inferiore a −100%, oppure un campo manca o è sconosciuto |
| `FM-E004` | La crescita di lungo periodo è pari o superiore al tasso |
| `FM-E018` | H è negativo |
| `FM-E008` | Overflow aritmetico (input molto lontani da un intervallo realistico) |

## Precisione

Solo operazioni razionali: risultato esatto fino alle 28 cifre significative del tipo
decimale. L'errore di approssimazione dell'H-model rispetto a un andamento esatto in calo è
una proprietà del modello e non viene corretto.

## Verificato con

- Casi di riferimento in `tests/golden/tv.h_model/`, da un calcolo indipendente a 50 cifre
  (`scripts/reference_tv.py`) e dalla forma chiusa riportata in ciascun file.
- Un test unitario verifica che crescite uguali diano esattamente il risultato di `tv.gordon`.
