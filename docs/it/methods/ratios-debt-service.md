---
source: methods/ratios-debt-service.md
source_hash: b9225adeacfe
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
\text{LLCR} = \frac{\sum_{t=1}^{n} \text{CFADS}_t \,/\, (1 + r)^t + \text{riserva}}{\text{debito residuo}}
$$

dove CFADS è il flusso di cassa disponibile per il servizio del debito, $r$ il tasso di
attualizzazione per periodo del piano (il costo del debito per periodo scelto dall'analista), la
riserva il saldo della riserva per il servizio del debito (DSRA) alla data di valutazione,
sommato senza attualizzazione (zero se non fornito), $n$ il numero di periodi fino alla scadenza finale e il
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

# Senza piano di rimborso DSCR e LLCR non sono definiti; gli altri indici sono calcolati.
s = fm.ratios.debt_service(
    balances="closing",
    ebitda_definition="reported EBITDA, no adjustments",
    interest_definition="gross interest expense",
    income_statement={"ebitda": 800, "ebit": 500, "interest": 100},
    closing={"net_financial_position": {"gross_debt": 2200, "cash": 300}},
)
assert s.llcr["not_defined"] == "no debt schedule given"
assert s.interest_cover_ebitda["value"] == Decimal("8")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `balances` | `closing`, oppure `average` della posizione finanziaria netta di apertura e di chiusura | obbligatorio |
| `ebitda_definition` | Come è stato misurato l'EBITDA, a parole, non vuoto se fornito; riportato nelle convenzioni | facoltativo: senza di esso gli indici sull'EBITDA non sono calcolati |
| `interest_definition` | Come sono stati misurati gli oneri finanziari (lordi, o al netto dei proventi finanziari), a parole, non vuoto se fornito; riportato nelle convenzioni | facoltativo: senza di esso le coperture degli oneri finanziari non sono calcolate |
| `income_statement` | `ebitda`, `ebit`, `interest` del periodo, in unità monetarie | ogni importo facoltativo |
| `closing` | `net_financial_position` con `gross_debt`, `cash` e `other_items` | ogni importo facoltativo |
| `opening` | Lo stesso campo all'inizio del periodo | usato con saldi medi, non usato altrimenti; ogni importo facoltativo |
| `debt_schedule` | `discount_rate`: il costo del debito per periodo scelto dall'analista (frazione decimale); `debt_outstanding` all'inizio del primo periodo; `reserve_balance` facoltativo (riserva per il servizio del debito alla data di valutazione); `periods`: un elenco di `{"cfads", "debt_service"}` fino alla scadenza finale, almeno uno | facoltativo: senza di esso DSCR e LLCR non sono definiti; ogni importo al suo interno facoltativo |

## Output

| Campo | Significato |
| --- | --- |
| `net_financial_position` | PFN usata dagli indici, in unità monetarie |
| `interest_cover_ebit` | EBIT / oneri finanziari, multiplo |
| `interest_cover_ebitda` | EBITDA / oneri finanziari, multiplo |
| `nfp_to_ebitda` | PFN / EBITDA, multiplo |
| `dscr` | CFADS / servizio del debito di ogni periodo, multiplo, nell'ordine del piano; vuoto senza piano di rimborso |
| `pv_cfads` | Valore attuale dei CFADS all'inizio del primo periodo, in unità monetarie; null senza piano di rimborso |
| `llcr` | (Valore attuale dei CFADS + riserva) / debito residuo, multiplo |
| `not_computed` | Output non calcolati perché manca un input, nell'ordine degli output: `{"output", "missing", "reason"}`, con `output` il JSON pointer dell'output nel valore del risultato e `missing` i JSON pointer ordinati degli input che gli mancano; vuoto quando ogni output è calcolato |

Gli indici sono oggetti `{"value": ..., "not_defined": ...}`, come in ogni gruppo di indici.

## Input parziale

Ogni importo è facoltativo (versione 2, SPEC-0007 R7). Si fornisce ciò che il bilancio riporta:
ogni output è calcolato quando sono forniti i suoi importi; altrimenti è null ed è elencato in
`not_computed` con i JSON pointer degli input che gli mancano. Un importo mancante non è mai
considerato zero, e un dato mancante non è mai un errore. Con i saldi medi un importo richiede sia
`/closing/<campo>` sia `/opening/<campo>`. Un importo fornito che rende già l'indice non definito
(per esempio un denominatore nullo) prevale: l'indice è allora non definito, con il suo motivo, e
non è elencato.

| Output | Input necessari |
| --- | --- |
| `net_financial_position` | debiti finanziari lordi, disponibilità liquide, l'importo di ogni altra voce fornita |
| `interest_cover_ebit` | EBIT, oneri finanziari, `interest_definition` |
| `interest_cover_ebitda` | EBITDA, `ebitda_definition`, oneri finanziari, `interest_definition` |
| `nfp_to_ebitda` | la posizione finanziaria netta, EBITDA, `ebitda_definition` |
| `dscr` (ogni periodo) | i CFADS e il servizio del debito del periodo |
| `pv_cfads` | `discount_rate` e i CFADS di ogni periodo |
| `llcr` | il valore attuale dei CFADS, `debt_outstanding` |

Senza piano di rimborso `dscr` è vuoto e `pv_cfads` e `llcr` sono elencati con
`/debt_schedule`; `llcr` mantiene il motivo della versione 1, "no debt schedule given".

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Saldi | di chiusura, media tra apertura e chiusura (posizione finanziaria netta) | obbligatorio |
| Definizione dell'EBITDA | come indicata dall'analista, non vuota | facoltativa; senza di essa gli indici sull'EBITDA non sono calcolati |
| Definizione degli oneri finanziari | come indicata dall'analista (lordi, o al netto dei proventi finanziari), non vuota | facoltativa; senza di essa le coperture degli oneri finanziari non sono calcolate |
| Servizio del debito | come fornito | fissa |
| Attualizzazione dell'LLCR | a fine periodo, a `discount_rate` per periodo, il costo del debito scelto dall'analista | fissa |
| Riserva | sommata senza attualizzazione al numeratore dell'LLCR | nessuna |
| Piano di rimborso | facoltativo; senza di esso DSCR e LLCR non sono definiti ("no debt schedule given") | nessuno |
| Non definito | denominatore nullo; copertura degli oneri finanziari quando gli oneri o il numeratore (EBIT o EBITDA) sono nulli o negativi; PFN / EBITDA quando l'EBITDA è nullo o negativo; DSCR quando il servizio del debito è nullo o negativo; LLCR quando il debito residuo è nullo o negativo | fissa |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | `balances` mancante; `ebitda_definition` o `interest_definition` vuoto o composto solo da spazi; un campo sconosciuto; un importo che non è un decimale |
| `FM-E002` | È fornito un piano di rimborso senza periodi |
| `FM-E003` | Tasso di attualizzazione pari o inferiore a −100% per periodo; il messaggio indica `/debt_schedule/discount_rate` |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Gli indici sono ciascuno una divisione decimale; i fattori di sconto sono potenze intere di
$1 + r$. I risultati sono esatti fino alle 28 cifre significative del tipo decimale.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/ratios.debt_service/`, i cui valori attesi
  provengono da `scripts/reference_ratios.py` (decimal di Python a 50 cifre); l'LLCR di un CFADS
  costante è pari alla forma chiusa della rendita $C\,(1 - (1 + r)^{-n}) / r$ divisa per il
  debito residuo. Altri casi
  coprono la riserva, l'assenza del piano di rimborso e ogni segno non significativo.
- Casi di riferimento con input parziale (oneri finanziari senza la loro definizione, un periodo
  senza servizio del debito, un piano di rimborso senza tasso di attualizzazione) e un confronto
  casuale con seme fisso su input parziali, rispetto a `scripts/reference_ratios_partial.py`.
