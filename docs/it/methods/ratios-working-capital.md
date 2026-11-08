---
source: methods/ratios-working-capital.md
source_hash: bfcc52677556
---
# Indici del capitale circolante (`ratios.working_capital`)

Risponde alla domanda "quanto capitale assorbe il circolante?". Restituisce i giorni medi di
incasso dei crediti (DSO), i giorni medi di giacenza delle scorte (DIO), i giorni medi di
pagamento dei fornitori (DPO) e il ciclo monetario (cash conversion cycle), su una base giorni
scelta dall'analista.

## Formula

$$
\text{DSO} = \frac{\text{crediti commerciali} \times \text{giorni}}{\text{ricavi}}
\qquad
\text{DIO} = \frac{\text{rimanenze} \times \text{giorni}}{\text{costo del venduto}}
\qquad
\text{DPO} = \frac{\text{debiti commerciali} \times \text{giorni}}{\text{acquisti}}
$$

$$
\text{ciclo monetario} = \text{DSO} + \text{DIO} - \text{DPO}
$$

dove i giorni sono 360 o 365, secondo la scelta in `day_basis`. I denominatori seguono il
curriculum del CFA Institute (crediti sui ricavi, rimanenze sul costo del venduto, debiti sugli
acquisti), che corrisponde anche alla prassi italiana più diffusa.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

r = fm.ratios.working_capital(
    balances="closing",
    day_basis=365,
    income_statement={"revenue": 5000, "cost_of_sales": 3000, "purchases": 2800},
    closing={"trade_receivables": 1200, "inventories": 600, "trade_payables": 700},
)
assert r.days_sales_outstanding["value"] == Decimal("87.6")  # 1200 × 365 / 5000
assert r.days_inventory_outstanding["value"] == Decimal("73")  # 600 × 365 / 3000
assert r.days_payable_outstanding["value"] == Decimal("91.25")  # 700 × 365 / 2800
assert r.cash_conversion_cycle["value"] == Decimal("69.35")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `balances` | `closing`, oppure `average` degli stati patrimoniali di apertura e di chiusura | obbligatorio |
| `day_basis` | `"360"` o `"365"` | obbligatorio |
| `income_statement` | `revenue`, `cost_of_sales`, `purchases` dell'anno, in unità monetarie | obbligatorio |
| `closing` | `trade_receivables`, `inventories`, `trade_payables`, in unità monetarie | obbligatorio |
| `opening` | Gli stessi campi all'inizio del periodo | obbligatorio con saldi medi, non usato altrimenti |

Gli importi sono usati come forniti: crediti commerciali e ricavi vanno espressi sulla stessa
base IVA, e così debiti commerciali e acquisti.

## Output

| Campo | Significato |
| --- | --- |
| `days_sales_outstanding` | Crediti commerciali × giorni / ricavi, giorni |
| `days_inventory_outstanding` | Rimanenze × giorni / costo del venduto, giorni |
| `days_payable_outstanding` | Debiti commerciali × giorni / acquisti, giorni |
| `cash_conversion_cycle` | DSO + DIO − DPO, giorni; non definito quando uno dei tre non lo è |

Gli indici sono oggetti `{"value": ..., "not_defined": ...}`, come in ogni gruppo di indici.

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Saldi | di chiusura, media tra apertura e chiusura | obbligatorio |
| Base giorni | 360, 365 | obbligatorio |
| Denominatori di DSO, DIO, DPO | ricavi, costo del venduto, acquisti | fissa |
| IVA | importi usati come forniti | fissa |
| Non definito | denominatore nullo | fissa |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Saldi medi senza `opening`; una base giorni diversa da 360 o 365; un campo mancante o sconosciuto |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Ogni indice moltiplica per i giorni prima di un'unica divisione, esatta fino alle 28 cifre
significative del tipo decimale.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/ratios.working_capital/` (saldi di chiusura
  su 365 e su 360 giorni, saldi medi su 360 giorni, acquisti nulli), i cui valori attesi provengono da
  `scripts/reference_ratios.py` (decimal di Python a 50 cifre), con la forma chiusa scritta in
  ciascun file.
