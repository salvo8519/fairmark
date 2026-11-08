---
source: methods/tv-value-driver.md
source_hash: 4593f2692a3a
---
# Valore terminale con la formula dei value driver (`tv.value_driver`)

Restituisce il valore terminale all'orizzonte N quando la crescita è ottenuta reinvestendo:
per crescere a g, un'azienda che ottiene un rendimento del nuovo capitale investito (RONIC)
deve reinvestire g / RONIC del proprio reddito operativo netto dopo le imposte (NOPAT). Il
flusso di cassa disponibile è ciò che resta. Il metodo rende esplicita la creazione di valore:
la crescita aggiunge valore solo quando il RONIC supera il tasso, e quando il RONIC è pari al
tasso il valore terminale non dipende da g.

## Formula

Per $-1 < g < r$ e $RONIC > 0$:

$$
\text{tasso di reinvestimento} = \frac{g}{RONIC},
\qquad
TV_N = \frac{NOPAT_{N+1}\,\left(1 - \dfrac{g}{RONIC}\right)}{r - g}
$$

Quando $RONIC = r$ la formula si riduce a $TV_N = NOPAT_{N+1} / r$; il metodo la calcola
direttamente in questa forma, quindi il risultato è esattamente indipendente da g. Quando il
RONIC è inferiore a g il tasso di reinvestimento supera il 100% e il flusso di cassa disponibile
in N + 1 è negativo; il metodo restituisce il valore e lo dichiara in `explain.conventions`.

## Esempio

La tabella "The Big Assumption" delle lezioni di valutazione di Damodaran valuta un NOPAT di
100 a un costo del capitale del 10%; con crescita del 2% e rendimento del capitale del 6%
riporta 833:

```python
from decimal import Decimal

import fairmark as fm

r = fm.tv.value_driver(100, "0.02", "0.06", "0.10")
assert round(r.terminal_value) == 833
assert r.value_of_growth == "destroys"

# RONIC pari al tasso: la crescita non crea valore, TV = NOPAT / r per ogni g
flat = fm.tv.value_driver(100, "0.03", "0.10", "0.10")
assert flat.terminal_value == Decimal("1000")
assert flat.value_of_growth == "none"

# NOPAT 150, crescita 2%, RONIC 12%: si reinveste un sesto del NOPAT
up = fm.tv.value_driver(150, "0.02", "0.12", "0.10")
assert round(up.reinvestment_rate, 6) == Decimal("0.166667")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `nopat_next` | NOPAT del periodo N + 1, in unità monetarie | obbligatorio |
| `growth` | Crescita perpetua, frazione decimale per periodo, maggiore di −1 e inferiore a `rate` | obbligatorio |
| `ronic` | Rendimento del nuovo capitale investito, frazione decimale per periodo, maggiore di 0 | obbligatorio |
| `rate` | Tasso di attualizzazione della fase terminale, frazione decimale per periodo | obbligatorio |
| `growth_cap` | Limite alla crescita, frazione decimale per periodo | nessuno, riportato come assente |

## Output

| Campo | Significato |
| --- | --- |
| `terminal_value` | Valore terminale in N, non attualizzato |
| `reinvestment_rate` | g / RONIC |
| `next_free_cash_flow` | NOPAT_{N+1} × (1 − g / RONIC) |
| `value_of_growth` | `creates` (RONIC superiore al tasso), `none` (pari) oppure `destroys` (inferiore) |
| `growth_cap` | `{growth, cap, status}`; `null` senza limite |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Crescita da reinvestimento | tasso di reinvestimento g / RONIC del NOPAT_{N+1} | fissa |
| Tasso di reinvestimento superiore al 100% | riportato in `explain.conventions`, non rifiutato | fisso |
| Tasso della fase terminale | fornito, può differire da quello della fase esplicita | nessuno: va fornito |
| Limite alla crescita | valore oppure nessuno | nessuno, riportato come assente |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | La crescita è pari o inferiore a −100%, oppure un campo manca o è sconosciuto |
| `FM-E004` | La crescita è pari o superiore al tasso |
| `FM-E018` | Il RONIC è pari a zero o negativo |
| `FM-E008` | Overflow aritmetico (input molto lontani da un intervallo realistico) |

## Precisione

Solo operazioni razionali: risultato esatto fino alle 28 cifre significative del tipo
decimale. Il tasso di reinvestimento è arrotondato alla 28a cifra quando non è un decimale
finito (ad esempio 1/6).

## Verificato con

- Quattro celle della tabella "The Big Assumption" (Damodaran, slide delle lezioni di
  valutazione NYU Stern: NOPAT 100 nell'anno n+1, costo del capitale 10%) coincidono alla
  precisione pubblicata, compresa la colonna in cui il rendimento del capitale è pari al costo
  del capitale e il valore è 1.000 per ogni crescita (`tests/golden/tv.value_driver/`).
- Un calcolo indipendente a 50 cifre (`scripts/reference_tv.py`).
