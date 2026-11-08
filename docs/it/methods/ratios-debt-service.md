---
source: methods/ratios-debt-service.md
source_hash: da56b631ae14
---
# Indici di servizio del debito (`ratios.debt_service`)

Risponde alla domanda "la società è in grado di servire il proprio debito?". Restituisce la
copertura degli oneri finanziari (interest cover) su EBIT e su EBITDA, il rapporto tra
posizione finanziaria netta ed EBITDA, il DSCR (indice di copertura del servizio del debito) di
ogni periodo di un piano di rimborso e l'LLCR (indice di copertura sulla durata del
finanziamento).

## Formula

$$
\text{interest cover} = \frac{\text{EBIT}}{\text{oneri finanziari}} \;\text{e}\; \frac{\text{EBITDA}}{\text{oneri finanziari}}
\qquad
\frac{\text{PFN}}{\text{EBITDA}}
$$

$$
\text{DSCR}_t = \frac{\text{CFADS}_t}{\text{servizio del debito}_t}
\qquad
\text{LLCR} = \frac{\sum_{t=1}^{n} \text{CFADS}_t \,/\, (1 + r)^t}{\text{debito residuo}}
$$

dove CFADS è il flusso di cassa disponibile per il servizio del debito, $r$ il tasso di
attualizzazione per periodo del piano, $n$ il numero di periodi fino alla scadenza finale e il
debito residuo è misurato all'inizio del primo periodo. L'attualizzazione è quella di
[`tvm.present_value`](tvm-present-value.md), a fine periodo.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

r = fm.ratios.debt_service(
    balances="closing",
    ebitda_definition="reported EBITDA, no adjustments",
    interest_definition="gross interest expense",
    income_statement={"ebitda": 800, "ebit": 500, "interest": 100},
    closing={"net_financial_position": {"gross_debt": 2200, "cash": 300}},
    debt_schedule={
        "discount_rate": "0.06",
        "debt_outstanding": 1000,
        "periods": [{"cfads": 250, "debt_service": 200}] * 5,
    },
)
assert r.interest_cover_ebit["value"] == Decimal("5")  # 500 / 100
assert r.nfp_to_ebitda["value"] == Decimal("2.375")  # 1900 / 800
assert r.dscr[0]["value"] == Decimal("1.25")  # 250 / 200
# CFADS costante: LLCR = 250 × (1 − 1,06^−5) / 0,06 / 1000
assert abs(r.llcr["value"] - Decimal("1.05309094639142847003375")) < Decimal("1e-20")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `balances` | `closing`, oppure `average` della posizione finanziaria netta di apertura e di chiusura | obbligatorio |
| `ebitda_definition` | Come è stato misurato l'EBITDA, a parole, non vuoto; riportato nelle convenzioni | obbligatorio |
| `interest_definition` | Come sono stati misurati gli oneri finanziari (lordi, o al netto dei proventi finanziari), a parole, non vuoto; riportato nelle convenzioni | obbligatorio |
| `income_statement` | `ebitda`, `ebit`, `interest` del periodo, in unità monetarie | obbligatorio |
| `closing` | `net_financial_position` con `gross_debt`, `cash` e `other_items` | obbligatorio |
| `opening` | Lo stesso campo all'inizio del periodo | obbligatorio con saldi medi, non usato altrimenti |
| `debt_schedule` | `discount_rate` per periodo (frazione decimale), `debt_outstanding` all'inizio del primo periodo, `periods`: un elenco di `{"cfads", "debt_service"}` fino alla scadenza finale | obbligatorio |

## Output

| Campo | Significato |
| --- | --- |
| `net_financial_position` | PFN usata dagli indici, in unità monetarie |
| `interest_cover_ebit` | EBIT / oneri finanziari, multiplo |
| `interest_cover_ebitda` | EBITDA / oneri finanziari, multiplo |
| `nfp_to_ebitda` | PFN / EBITDA, multiplo |
| `dscr` | CFADS / servizio del debito di ogni periodo, multiplo, nell'ordine del piano |
| `pv_cfads` | Valore attuale dei CFADS all'inizio del primo periodo, in unità monetarie |
| `llcr` | Valore attuale dei CFADS / debito residuo, multiplo |

Gli indici sono oggetti `{"value": ..., "not_defined": ...}`, come in ogni gruppo di indici.

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Saldi | di chiusura, media tra apertura e chiusura (posizione finanziaria netta) | obbligatorio |
| Definizione dell'EBITDA | come indicata dall'analista, non vuota | obbligatorio |
| Definizione degli oneri finanziari | come indicata dall'analista (lordi, o al netto dei proventi finanziari), non vuota | obbligatorio |
| Servizio del debito | come fornito | fissa |
| Attualizzazione dell'LLCR | a fine periodo, a `discount_rate` per periodo | fissa |
| Non definito | denominatore nullo (oneri finanziari, EBITDA, servizio del debito di un periodo, debito residuo) | fissa |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Saldi medi senza `opening`; `ebitda_definition` o `interest_definition` vuoto o composto solo da spazi; un campo mancante o sconosciuto |
| `FM-E002` | Il piano di rimborso non ha periodi |
| `FM-E003` | Tasso di attualizzazione pari o inferiore a −100% per periodo |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Gli indici sono ciascuno una divisione decimale; i fattori di sconto sono potenze intere di
$1 + r$. I risultati sono esatti fino alle 28 cifre significative del tipo decimale.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/ratios.debt_service/`, i cui valori attesi
  provengono da `scripts/reference_ratios.py` (decimal di Python a 50 cifre); l'LLCR di un CFADS
  costante è pari alla forma chiusa della rendita $C\,(1 - (1 + r)^{-n}) / r$ divisa per il
  debito residuo.
