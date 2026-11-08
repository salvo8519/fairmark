---
source: methods/ratios-profitability.md
source_hash: 862988b084a5
---
# Indici di redditività (`ratios.profitability`)

Risponde alla domanda "la società rende abbastanza su ciò che impiega?". Restituisce ROE, ROI,
ROS, EBITDA margin ed EBIT margin, la rotazione del capitale investito e la scomposizione del
ROE in tre fattori, ciascuno con la formula e i numeri sostituiti nella tracciabilità del
calcolo (audit trail). Un indice che non si può calcolare è riportato come non definito con il
motivo, e gli altri indici sono comunque calcolati.

## Formula

$$
\text{ROE} = \frac{\text{utile netto}}{\text{capitale proprio}}
\qquad
\text{ROI} = \frac{\text{EBIT}}{\text{base del rendimento operativo}}
\qquad
\text{ROS} = \frac{\text{EBIT}}{\text{ricavi}}
$$

$$
\text{rotazione del capitale investito} = \frac{\text{ricavi}}{\text{base del rendimento operativo}}
\qquad
\text{ROI} = \text{ROS} \times \text{rotazione del capitale investito}
$$

$$
\text{EBITDA margin} = \frac{\text{EBITDA}}{\text{ricavi}}
\qquad
\text{EBIT margin} = \frac{\text{EBIT}}{\text{ricavi}}
$$

La scomposizione in tre fattori (DuPont):

$$
\text{ROE} = \underbrace{\frac{\text{utile netto}}{\text{ricavi}}}_{\text{margine netto}}
\times \underbrace{\frac{\text{ricavi}}{\text{totale attivo}}}_{\text{rotazione dell'attivo}}
\times \underbrace{\frac{\text{totale attivo}}{\text{capitale proprio}}}_{\text{equity multiplier}}
$$

La base del rendimento operativo è il capitale investito operativo oppure il totale attivo,
secondo la scelta in `operating_return_base`. Gli importi di stato patrimoniale sono i saldi di
chiusura, oppure la media tra i saldi di apertura e di chiusura, secondo la scelta in
`balances`.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

r = fm.ratios.profitability(
    balances="closing",
    operating_return_base="invested_capital",
    ebitda_definition="reported EBITDA, no adjustments",
    income_statement={"revenue": 5000, "ebitda": 800, "ebit": 500, "net_income": 260},
    closing={"equity": 2000, "total_assets": 6000, "invested_capital": 4000},
)
assert r.roe["value"] == Decimal("0.13")  # 260 / 2000
assert r.roi["value"] == Decimal("0.125")  # 500 / 4000
assert r.capital_turnover["value"] == Decimal("1.25")  # 5000 / 4000

# Capitale proprio negativo: il ROE non è definito, gli altri indici sono calcolati.
n = fm.ratios.profitability(
    balances="closing",
    operating_return_base="total_assets",
    ebitda_definition="reported EBITDA, no adjustments",
    income_statement={"revenue": 5000, "ebitda": 800, "ebit": 500, "net_income": 260},
    closing={"equity": -100, "total_assets": 6000},
)
assert n.roe["value"] is None
print(n.roe["not_defined"])  # equity is negative (-100)
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `balances` | `closing`, oppure `average` degli stati patrimoniali di apertura e di chiusura | obbligatorio |
| `operating_return_base` | `invested_capital` o `total_assets`: il capitale del ROI e della rotazione | obbligatorio |
| `ebitda_definition` | Come è stato misurato l'EBITDA, a parole; riportato nelle convenzioni | obbligatorio |
| `income_statement` | `revenue`, `ebitda`, `ebit`, `net_income` del periodo, in unità monetarie | obbligatorio |
| `closing` | `equity`, `total_assets` e, con base capitale investito, `invested_capital`, in unità monetarie | obbligatorio |
| `opening` | Gli stessi campi all'inizio del periodo | obbligatorio con saldi medi, non usato altrimenti |

## Output

Ogni indice è un oggetto `{"value": ..., "not_defined": ...}`: `value` è null quando l'indice
non è definito, e `not_defined` ne indica allora il motivo.

| Campo | Significato |
| --- | --- |
| `roe` | Utile netto / capitale proprio, frazione decimale |
| `roi` | EBIT / base del rendimento operativo, frazione decimale |
| `ros` | EBIT / ricavi, frazione decimale |
| `ebitda_margin` | EBITDA / ricavi, frazione decimale |
| `ebit_margin` | EBIT / ricavi, frazione decimale |
| `capital_turnover` | Ricavi / base del rendimento operativo, volte nel periodo |
| `roe_three_factor` | `net_margin`, `asset_turnover`, `equity_multiplier` e il loro prodotto `roe` |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Saldi | di chiusura, media tra apertura e chiusura | obbligatorio |
| Base del rendimento operativo | capitale investito, totale attivo | obbligatorio |
| Definizione dell'EBITDA | come indicata dall'analista | obbligatorio |
| Non definito | denominatore nullo; capitale proprio nullo o negativo per ROE ed equity multiplier | fissa |

Il ROS è l'EBIT sui ricavi, quindi coincide con l'EBIT margin; sono restituiti entrambi perché
la catena ROI = ROS × rotazione del capitale investito si legga direttamente.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Saldi medi senza `opening`; `invested_capital` mancante con base capitale investito; un campo mancante o sconosciuto |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Ogni indice è una divisione decimale, esatta fino alle 28 cifre significative del tipo
decimale. Il ROE in tre fattori è il prodotto dei tre fattori arrotondati e può differire dal
ROE diretto alla ventottesima cifra.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/ratios.profitability/` (saldi di chiusura e
  saldi medi, capitale proprio negativo, ricavi nulli), i cui valori attesi provengono da
  `scripts/reference_ratios.py` (decimal di Python a 50 cifre), con la forma chiusa scritta in
  ciascun file.
