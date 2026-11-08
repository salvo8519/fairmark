---
source: methods/tv-exit-multiple.md
source_hash: e8f1d96534fb
---
# Valore terminale con il multiplo di uscita (`tv.exit_multiple`)

Restituisce il valore terminale all'orizzonte N come multiplo di mercato di una grandezza
dell'anno N, come se l'azienda fosse ceduta in N. La grandezza determina l'approccio: EV/EBITDA,
EV/EBIT ed EV/Sales danno un enterprise value (approccio asset side); il P/E dà un equity value
(approccio equity side). I due approcci non si mescolano mai. Con il tasso e il flusso
successivo sullo stesso approccio, il metodo riporta anche la crescita perpetua implicita nel
multiplo, che rende confrontabile un'ipotesi di mercato con un'ipotesi di crescita.

## Formula

$$
TV_N = \text{multiplo} \times \text{grandezza}_N,
\qquad
g_{\text{implicita}} = r - \frac{CF_{N+1}}{TV_N}
$$

La crescita implicita è quella con cui [`tv.gordon`](tv-gordon.md), su $CF_{N+1}$ e $r$,
restituisce lo stesso valore terminale.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

# 8 × EBITDA 125, WACC 10%, FCFF in N + 1 pari a 60: il multiplo implica una crescita perpetua del 4%
r = fm.tv.exit_multiple(8, "ebitda", 125, rate="0.10", next_flow=60, growth_cap="0.03")
assert r.terminal_value == Decimal("1000")
assert r.basis == "enterprise"
assert r.implied_growth == Decimal("0.04")
assert r.growth_cap["status"] == "exceeded"

# Riportata nel modello di Gordon, la crescita implicita restituisce lo stesso valore
g = fm.tv.gordon("0.10", r.implied_growth, next_flow=60)
assert g.terminal_value == r.terminal_value

# P/E: un equity value
pe = fm.tv.exit_multiple(12, "earnings", 50)
assert (pe.terminal_value, pe.basis) == (Decimal("600"), "equity")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `multiple` | Il multiplo, maggiore di 0 | obbligatorio |
| `metric` | `ebitda`, `ebit`, `revenue` (asset side) oppure `earnings` (equity side) | obbligatorio |
| `metric_value` | La grandezza in N, in unità monetarie, maggiore di 0 | obbligatorio |
| `rate` | Tasso di attualizzazione sull'approccio della grandezza (WACC o costo del capitale proprio), frazione decimale per periodo; insieme a `next_flow` | nessuno |
| `next_flow` | Flusso in N + 1 sull'approccio della grandezza (FCFF o FCFE), in unità monetarie; insieme a `rate` | nessuno |
| `growth_cap` | Limite alla crescita implicita, frazione decimale per periodo; richiede `rate` e `next_flow` | nessuno, riportato come assente |

## Output

| Campo | Significato |
| --- | --- |
| `terminal_value` | Valore terminale in N, non attualizzato |
| `basis` | `enterprise` oppure `equity` |
| `implied_growth` | $r - CF_{N+1} / TV_N$; `null` senza `rate` e `next_flow` |
| `growth_cap` | La crescita implicita confrontata con il limite; `null` senza limite |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Grandezza e approccio | ebitda, ebit, revenue (asset side); earnings (equity side) | nessuno: va scelto |
| Tasso e flusso successivo della crescita implicita | sull'approccio della grandezza | non calcolata se assenti |
| Limite alla crescita | valore oppure nessuno | nessuno, riportato come assente |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | È fornito solo uno tra `rate` e `next_flow`, `growth_cap` è fornito senza di essi, la grandezza è sconosciuta, oppure un campo manca o è sconosciuto |
| `FM-E018` | Il multiplo o la grandezza è pari a zero o negativo |
| `FM-E008` | Overflow aritmetico (input molto lontani da un intervallo realistico) |

## Precisione

Una moltiplicazione per il valore, una divisione e una sottrazione per la crescita implicita:
risultato esatto fino alle 28 cifre significative del tipo decimale. Riportata in
`tv.gordon`, la crescita implicita restituisce lo stesso valore terminale entro $10^{-18}$
(l'arrotondamento di una divisione).

## Verificato con

- Casi di riferimento in `tests/golden/tv.exit_multiple/`, da forme chiuse e da un calcolo
  indipendente a 50 cifre (`scripts/reference_tv.py`).
- Test unitari e Python riportano la crescita implicita in `tv.gordon` su diversi input.
