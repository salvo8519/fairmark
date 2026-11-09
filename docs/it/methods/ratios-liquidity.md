---
source: methods/ratios-liquidity.md
source_hash: dc8fd64f93b5
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
| `closing` | `current_assets` (rimanenze e disponibilità liquide incluse), `inventories`, `cash`, `current_liabilities`, in unità monetarie | ogni importo facoltativo |
| `opening` | Gli stessi campi all'inizio del periodo | usato con saldi medi, non usato altrimenti; ogni importo facoltativo |

## Output

| Campo | Significato |
| --- | --- |
| `current_ratio` | Attivo corrente / passività correnti, multiplo |
| `quick_ratio` | (Attivo corrente − rimanenze) / passività correnti, multiplo |
| `cash_ratio` | Disponibilità liquide / passività correnti, multiplo |
| `net_working_capital` | Attivo corrente − passività correnti, in unità monetarie |
| `treasury_margin` | Attivo corrente − rimanenze − passività correnti, in unità monetarie |
| `not_computed` | Output non calcolati perché manca un input, nell'ordine degli output: `{"output", "missing", "reason"}`, con `output` il JSON pointer dell'output nel valore del risultato e `missing` i JSON pointer ordinati degli input che gli mancano; vuoto quando ogni output è calcolato |

Gli indici sono oggetti `{"value": ..., "not_defined": ...}`, come in ogni gruppo di indici; i
margini sono importi, null quando non calcolati.

## Input parziale

Ogni importo è facoltativo (versione 2, SPEC-0007 R7). Si fornisce ciò che il bilancio riporta:
ogni output è calcolato quando sono forniti i suoi importi; altrimenti è null ed è elencato in
`not_computed` con i JSON pointer degli input che gli mancano. Un importo mancante non è mai
considerato zero e da solo non è un errore. La soglia minima: almeno un output deve avere tutti
gli input che gli servono (conta anche un indice non definito dai propri input); sotto la soglia
l'input è rifiutato con `FM-E027`, il cui messaggio elenca gli insiemi alternativi di input che
la superano, così non si restituisce mai un risultato fatto solo di null. Con i saldi medi un importo richiede sia
`/closing/<campo>` sia `/opening/<campo>`. Un importo fornito che rende già l'indice non definito
(per esempio un denominatore nullo) prevale: l'indice è allora non definito, con il suo motivo, e
non è elencato.

| Output | Input necessari |
| --- | --- |
| `current_ratio`, `net_working_capital` | attivo corrente, passività correnti |
| `quick_ratio`, `treasury_margin` | attivo corrente, rimanenze, passività correnti |
| `cash_ratio` | disponibilità liquide, passività correnti |

```python
r = fm.ratios.liquidity(
    balances="closing",
    closing={"current_assets": 2500, "cash": 200, "current_liabilities": 2000},
)
assert r.current_ratio["value"] == Decimal("1.25")
assert r.quick_ratio["value"] is None  # rimanenze non fornite
assert r.not_computed[0] == {
    "output": "/quick_ratio",
    "missing": ["/closing/inventories"],
    "reason": "an input it needs is not given; a missing input is never taken as zero",
}
```

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Saldi | di chiusura, media tra apertura e chiusura | obbligatorio |
| Non definito | passività correnti nulle | fissa |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | `balances` mancante o diverso da `closing` e `average`; un campo sconosciuto; un importo che non è un decimale |
| `FM-E027` | Sotto la soglia minima di input: nessun output ha tutti gli input che gli servono; il messaggio elenca gli insiemi alternativi di input (ne basta uno) |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

I margini sono esatti; ogni indice è una divisione decimale, esatta fino alle 28 cifre
significative del tipo decimale.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/ratios.liquidity/` (saldi di chiusura e
  saldi medi, passività correnti nulle), i cui valori attesi provengono da `scripts/reference_ratios.py`
  (decimal di Python a 50 cifre), con la forma chiusa scritta in ciascun file.
- Casi di riferimento con input parziale (rimanenze non fornite, un importo di apertura non
  fornito) e un confronto casuale con seme fisso su input parziali, rispetto a
  `scripts/reference_ratios_partial.py`.
