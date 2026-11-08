---
source: methods/ratios-profitability.md
source_hash: 13f455e5c636
---
# Indici di redditività (`ratios.profitability`)

Risponde alla domanda "la società rende abbastanza su ciò che impiega?". Restituisce ROE, ROI,
ROA, ROS, EBITDA margin ed EBIT margin, la rotazione del capitale investito e la scomposizione
del ROE in tre fattori e nella forma con leva finanziaria, ciascuno con la formula e i numeri sostituiti nella tracciabilità del
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

$$
\text{ROA} = \frac{\text{EBIT}}{\text{totale attivo}}
$$

La forma con leva finanziaria, calcolata quando sono forniti gli input di `leverage`:

$$
\text{ROE} = \left[\text{ROI} + (\text{ROI} - i)\,\frac{D}{E}\right](1 - t)
\qquad
i = \frac{\text{oneri finanziari}}{D}
\qquad
t = \frac{\text{imposte}}{\text{risultato ante imposte}}
$$

dove $D$ sono i debiti finanziari lordi oppure la posizione finanziaria netta, secondo la scelta
in `leverage.debt_basis`, $E$ il capitale proprio e $t$ l'aliquota fiscale effettiva.
$(\text{ROI} - i) \times D/E$ è l'effetto leva. La forma coincide con il ROE quando la base del
rendimento operativo è il capitale investito e il capitale investito è pari a $E + D$;
altrimenti la differenza mostra l'effetto delle componenti non operative.

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

# Forma con leva finanziaria, D sulla posizione finanziaria netta: coincide con il ROE quando
# il capitale investito è pari a E + D.
lev = fm.ratios.profitability(
    balances="closing",
    operating_return_base="invested_capital",
    ebitda_definition="reported EBITDA, no adjustments",
    income_statement={"revenue": 5000, "ebitda": 800, "ebit": 500, "net_income": 260},
    closing={"equity": 2000, "total_assets": 6000, "invested_capital": 4000, "debt": 2000},
    leverage={
        "debt_basis": "net_financial_position",
        "financial_charges": 100,
        "taxes": 140,
        "pre_tax_income": 400,
    },
)
assert lev.roe_leverage["cost_of_debt"]["value"] == Decimal("0.05")  # 100 / 2000
assert lev.roe_leverage["leverage_effect"]["value"] == Decimal("0.075")  # (0,125 − 0,05) × 1
assert lev.roe_leverage["roe"]["value"] == Decimal("0.13")  # 0,2 × (1 − 0,35)
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `balances` | `closing`, oppure `average` degli stati patrimoniali di apertura e di chiusura | obbligatorio |
| `operating_return_base` | `invested_capital` o `total_assets`: il capitale del ROI e della rotazione | obbligatorio |
| `ebitda_definition` | Come è stato misurato l'EBITDA, a parole, non vuoto; riportato nelle convenzioni | obbligatorio |
| `income_statement` | `revenue`, `ebitda`, `ebit`, `net_income` del periodo, in unità monetarie | obbligatorio |
| `closing` | `equity`, `total_assets`; con base capitale investito, `invested_capital`; con `leverage`, `debt` (D sulla base scelta); in unità monetarie | obbligatorio |
| `opening` | Gli stessi campi all'inizio del periodo | obbligatorio con saldi medi, non usato altrimenti |
| `leverage` | `debt_basis` (`gross_financial_debt` o `net_financial_position`, senza default), `financial_charges`, `taxes`, `pre_tax_income` del periodo, in unità monetarie | facoltativo: senza di esso la forma con leva finanziaria non è definita |

## Output

Ogni indice è un oggetto `{"value": ..., "not_defined": ...}`: `value` è null quando l'indice
non è definito, e `not_defined` ne indica allora il motivo.

| Campo | Significato |
| --- | --- |
| `roe` | Utile netto / capitale proprio, frazione decimale |
| `roi` | EBIT / base del rendimento operativo, frazione decimale |
| `roa` | EBIT / totale attivo, frazione decimale |
| `ros` | EBIT / ricavi, frazione decimale |
| `ebitda_margin` | EBITDA / ricavi, frazione decimale |
| `ebit_margin` | EBIT / ricavi, frazione decimale |
| `capital_turnover` | Ricavi / base del rendimento operativo, volte nel periodo |
| `roe_three_factor` | `net_margin`, `asset_turnover`, `equity_multiplier` e il loro prodotto `roe` |
| `roe_leverage` | `roi`, `cost_of_debt` (i), `debt_to_equity` (D/E), `leverage_effect` (effetto leva), `pre_tax_roe`, `tax_rate` (t) e `roe` |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Saldi | di chiusura, media tra apertura e chiusura | obbligatorio |
| Base del rendimento operativo | capitale investito, totale attivo; un capitale investito fornito con base totale attivo è segnalato come non usato | obbligatorio |
| Definizione dell'EBITDA | come indicata dall'analista, non vuota | obbligatorio |
| ROS | EBIT / ricavi, pari all'EBIT margin | fissa |
| ROA | EBIT / totale attivo | fissa |
| Forma con leva finanziaria | D secondo `debt_basis`; i = oneri finanziari / D; t = imposte / risultato ante imposte (aliquota effettiva); ROI sulla base del rendimento operativo; `debt` fornito senza `leverage` è segnalato come non usato | `debt_basis` obbligatorio quando `leverage` è fornito |
| ROE in tre fattori | sul totale attivo (rotazione dell'attivo ed equity multiplier), qualunque sia la base del rendimento operativo | fissa |
| Saldi | applicati a ogni importo di stato patrimoniale, al numeratore o al denominatore, D compreso | fissa |
| Non definito | denominatore nullo; capitale proprio nullo o negativo per ROE, equity multiplier e D/E; capitale investito nullo o negativo per il ROI su base capitale investito; input di `leverage` assenti per la forma con leva finanziaria | fissa |

Il ROS è l'EBIT sui ricavi, quindi coincide con l'EBIT margin; sono restituiti entrambi perché
la catena ROI = ROS × rotazione del capitale investito si legga direttamente.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Saldi medi senza `opening`; `invested_capital` mancante con base capitale investito; `debt` o `debt_basis` mancante con `leverage`; `ebitda_definition` vuoto o composto solo da spazi; un campo mancante o sconosciuto |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Ogni indice è una divisione decimale, esatta fino alle 28 cifre significative del tipo
decimale. Il ROE in tre fattori è il prodotto dei tre fattori arrotondati e può differire dal
ROE diretto alla ventottesima cifra.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/ratios.profitability/` (saldi di chiusura e
  saldi medi su entrambe le basi del rendimento operativo, capitale proprio negativo, capitale
  investito negativo, ricavi nulli, forma con leva finanziaria su entrambe le basi di D e con
  risultato ante imposte nullo), i cui valori attesi provengono da
  `scripts/reference_ratios.py` (decimal di Python a 50 cifre), con la forma chiusa scritta in
  ciascun file.
