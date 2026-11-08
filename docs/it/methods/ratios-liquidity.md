---
source: methods/ratios-liquidity.md
source_hash: a80dc9ec0722
---
# Indici di liquidità (`ratios.liquidity`)

Risponde alla domanda "la società è in grado di pagare nel breve termine?". Restituisce
l'indice di disponibilità (current ratio), l'indice di liquidità primaria (quick ratio, acid
test), il cash ratio (disponibilità liquide su passività correnti), il capitale circolante netto
e il margine di tesoreria.

## Formula

$$
\text{current ratio} = \frac{\text{attivo corrente}}{\text{passività correnti}}
\qquad
\text{quick ratio} = \frac{\text{attivo corrente} - \text{rimanenze}}{\text{passività correnti}}
\qquad
\text{cash ratio} = \frac{\text{disponibilità liquide}}{\text{passività correnti}}
$$

$$
\text{capitale circolante netto} = \text{attivo corrente} - \text{passività correnti}
$$

$$
\text{margine di tesoreria} = \text{attivo corrente} - \text{rimanenze} - \text{passività correnti}
$$

## Esempio

```python
from decimal import Decimal

import fairmark as fm

r = fm.ratios.liquidity(
    balances="closing",
    closing={
        "current_assets": 2500,
        "inventories": 900,
        "cash": 200,
        "current_liabilities": 2000,
    },
)
assert r.current_ratio["value"] == Decimal("1.25")  # 2500 / 2000
assert r.quick_ratio["value"] == Decimal("0.8")  # (2500 − 900) / 2000
assert r.treasury_margin == Decimal("-400")  # 2500 − 900 − 2000
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `balances` | `closing`, oppure `average` degli stati patrimoniali di apertura e di chiusura | obbligatorio |
| `closing` | `current_assets` (rimanenze e disponibilità liquide incluse), `inventories`, `cash`, `current_liabilities`, in unità monetarie | obbligatorio |
| `opening` | Gli stessi campi all'inizio del periodo | obbligatorio con saldi medi, non usato altrimenti |

## Output

| Campo | Significato |
| --- | --- |
| `current_ratio` | Attivo corrente / passività correnti, multiplo |
| `quick_ratio` | (Attivo corrente − rimanenze) / passività correnti, multiplo |
| `cash_ratio` | Disponibilità liquide / passività correnti, multiplo |
| `net_working_capital` | Attivo corrente − passività correnti, in unità monetarie |
| `treasury_margin` | Attivo corrente − rimanenze − passività correnti, in unità monetarie |

Gli indici sono oggetti `{"value": ..., "not_defined": ...}`, come in ogni gruppo di indici.

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Saldi | di chiusura, media tra apertura e chiusura | obbligatorio |
| Non definito | passività correnti nulle | fissa |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Saldi medi senza `opening`; un campo mancante o sconosciuto |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

I margini sono esatti; ogni indice è una divisione decimale, esatta fino alle 28 cifre
significative del tipo decimale.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/ratios.liquidity/` (saldi di chiusura e
  saldi medi, passività correnti nulle), i cui valori attesi provengono da `scripts/reference_ratios.py`
  (decimal di Python a 50 cifre), con la forma chiusa scritta in ciascun file.
