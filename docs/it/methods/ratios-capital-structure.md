---
source: methods/ratios-capital-structure.md
source_hash: 95e8be0500bc
---
# Indici di struttura finanziaria (`ratios.capital_structure`)

Risponde alla domanda "la società è finanziata in modo equilibrato?". Restituisce il rapporto
tra debito e capitale proprio (D/E), il rapporto tra posizione finanziaria netta e capitale
proprio, l'indice di indipendenza finanziaria (equity ratio), il margine primario e il margine
secondario di struttura e i quozienti di struttura primario e secondario (copertura delle
immobilizzazioni). La posizione finanziaria netta è fornita con la sua composizione, e ogni
componente è riportata nella tracciabilità del calcolo.

## Formula

$$
\text{PFN} = \text{debiti finanziari lordi} - \text{disponibilità liquide} + \sum \text{altre voci}
$$

$$
\text{D/E} = \frac{\text{debiti finanziari lordi}}{\text{capitale proprio}}
\qquad
\text{PFN/E} = \frac{\text{PFN}}{\text{capitale proprio}}
\qquad
\text{equity ratio} = \frac{\text{capitale proprio}}{\text{totale attivo}}
$$

$$
\text{margine primario di struttura} = \text{capitale proprio} - \text{immobilizzazioni}
$$

$$
\text{margine secondario di struttura} = \text{capitale proprio} + \text{passività consolidate} - \text{immobilizzazioni}
$$

$$
\text{quoziente primario} = \frac{\text{capitale proprio}}{\text{immobilizzazioni}}
\qquad
\text{quoziente secondario} = \frac{\text{capitale proprio} + \text{passività consolidate}}{\text{immobilizzazioni}}
$$

Una posizione finanziaria netta positiva indica indebitamento netto. Fairmark somma le voci
fornite e non decide quali voci appartengono alla posizione finanziaria netta.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

r = fm.ratios.capital_structure(
    balances="closing",
    closing={
        "equity": 2000,
        "total_assets": 6000,
        "fixed_assets": 3500,
        "non_current_liabilities": 1800,
        "net_financial_position": {
            "gross_debt": 2200,
            "cash": 300,
            "other_items": [{"name": "lease liabilities", "amount": 150}],
        },
    },
)
assert r.net_financial_position == Decimal("2050")  # 2200 − 300 + 150
assert r.debt_to_equity["value"] == Decimal("1.1")  # 2200 / 2000
assert r.nfp_to_equity["value"] == Decimal("1.025")  # 2050 / 2000
assert r.primary_structure_margin == Decimal("-1500")  # 2000 − 3500
assert r.secondary_structure_margin == Decimal("300")  # 2000 + 1800 − 3500
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `balances` | `closing`, oppure `average` degli stati patrimoniali di apertura e di chiusura | obbligatorio |
| `closing` | `equity`, `total_assets`, `fixed_assets`, `non_current_liabilities`, `net_financial_position`, in unità monetarie | obbligatorio |
| `opening` | Gli stessi campi all'inizio del periodo | obbligatorio con saldi medi, non usato altrimenti |

`net_financial_position` contiene `gross_debt`, `cash` e `other_items`, un elenco di
`{"name", "amount"}` sommati con il loro segno (positivo per le voci assimilabili al debito,
negativo per le attività finanziarie; default: nessuna).

## Output

| Campo | Significato |
| --- | --- |
| `net_financial_position` | PFN usata dagli indici, in unità monetarie |
| `debt_to_equity` | Debiti finanziari lordi / capitale proprio, multiplo |
| `nfp_to_equity` | PFN / capitale proprio, multiplo |
| `equity_ratio` | Capitale proprio / totale attivo, frazione decimale |
| `primary_structure_margin` | Capitale proprio − immobilizzazioni, in unità monetarie |
| `secondary_structure_margin` | Capitale proprio + passività consolidate − immobilizzazioni, in unità monetarie |
| `primary_fixed_asset_coverage` | Capitale proprio / immobilizzazioni, multiplo |
| `secondary_fixed_asset_coverage` | (Capitale proprio + passività consolidate) / immobilizzazioni, multiplo |

Gli indici sono oggetti `{"value": ..., "not_defined": ...}`, come in ogni gruppo di indici.

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Saldi | di chiusura, media tra apertura e chiusura (applicata a ogni saldo, PFN inclusa) | obbligatorio |
| Posizione finanziaria netta | debiti finanziari lordi − disponibilità liquide + altre voci, positiva se indebitamento netto | fissa |
| Non definito | denominatore nullo; capitale proprio nullo o negativo per D/E e PFN/E | fissa |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Saldi medi senza `opening`; un campo mancante o sconosciuto |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

I margini sono esatti; ogni indice è una divisione decimale, esatta fino alle 28 cifre
significative del tipo decimale.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/ratios.capital_structure/` (saldi di
  chiusura e saldi medi, capitale proprio negativo), i cui valori attesi provengono da
  `scripts/reference_ratios.py` (decimal di Python a 50 cifre), con la forma chiusa scritta in
  ciascun file.
