---
source: methods/tv-normalised-income.md
source_hash: d0de776f5067
---
# Valore terminale con la capitalizzazione del reddito normalizzato (`tv.normalised_income`)

Restituisce il valore terminale all'orizzonte N come reddito normalizzato capitalizzato a un
tasso i, secondo il ragionamento reddituale della prassi italiana: il reddito R che l'azienda
può ottenere in modo sostenibile dopo l'orizzonte, diviso per il tasso di capitalizzazione,
con o senza una crescita perpetua. La stima di R e di i è esterna al metodo.

## Formula

$$
TV_N = \frac{R}{i}
\qquad\text{oppure, con una crescita } g < i,\qquad
TV_N = \frac{R}{i - g}
$$

## Esempio

```python
from decimal import Decimal

import fairmark as fm

r = fm.tv.normalised_income(120, "0.08")
assert r.terminal_value == Decimal("1500")
print(r.conventions[2])  # no growth: R / i (default)

grown = fm.tv.normalised_income(120, "0.09", growth="0.01", growth_cap="0.02")
assert grown.terminal_value == Decimal("1500")
assert grown.growth_cap["status"] == "respected"
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `income` | Reddito normalizzato R del periodo N + 1, in unità monetarie | obbligatorio |
| `capitalisation_rate` | Tasso di capitalizzazione i, frazione decimale per periodo, maggiore di 0 e della crescita | obbligatorio |
| `growth` | Crescita perpetua del reddito, frazione decimale per periodo, maggiore di −1 e inferiore a i | nessuna (R / i) |
| `growth_cap` | Limite alla crescita, frazione decimale per periodo | nessuno, riportato come assente |

## Output

| Campo | Significato |
| --- | --- |
| `terminal_value` | Valore terminale in N, non attualizzato |
| `growth_cap` | La crescita (0 se non fornita) confrontata con il limite; `null` senza limite |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Crescita | nessuna (R / i), oppure fornita (R / (i − g)) | nessuna, riportata come default |
| Tasso di capitalizzazione | fornito | nessuno: va fornito |
| Limite alla crescita | valore oppure nessuno | nessuno, riportato come assente |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | La crescita è pari o inferiore a −100%, oppure un campo manca o è sconosciuto |
| `FM-E004` | La crescita è pari o superiore al tasso di capitalizzazione |
| `FM-E018` | Il tasso di capitalizzazione è pari a zero o negativo (verificato per primo) |
| `FM-E008` | Overflow aritmetico (input molto lontani da un intervallo realistico) |

## Precisione

Una divisione (e una sottrazione con la crescita): risultato esatto fino alle 28 cifre
significative del tipo decimale.

## Verificato con

- Casi di riferimento in `tests/golden/tv.normalised_income/`, da forme chiuse riportate in
  ciascun file e da un calcolo indipendente a 50 cifre (`scripts/reference_tv.py`).
