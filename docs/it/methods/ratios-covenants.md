---
source: methods/ratios-covenants.md
source_hash: ff844a5c41a8
---
# Verifica dei covenant (`ratios.covenants`)

Risponde alla domanda "la società rispetta i propri covenant?". Verifica un elenco di covenant
finanziari, ciascuno sul numeratore e sul denominatore definiti dal contratto di finanziamento,
rispetto a una soglia a una data di verifica, e restituisce per ogni covenant un esito esplicito
(`passed`, rispettato; `breached`, violato; `not_tested`, non verificato, con il motivo), il
valore verificato e l'headroom (margine rispetto alla soglia).

Il metodo non calcola gli indici standard degli altri gruppi: un covenant è verificato sulle
definizioni del contratto (per esempio "EBITDA consolidato dei 12 mesi che terminano alla data
di verifica, prima delle componenti non ricorrenti"), che spesso differiscono dai dati di
bilancio. L'analista calcola gli importi secondo quelle definizioni e li fornisce con le parole
del contratto; Fairmark riporta entrambi nella tracciabilità del calcolo (audit trail).

## Formula

$$
\text{valore verificato} = \frac{\text{numeratore}}{\text{denominatore}}
$$

$$
\text{headroom} =
\begin{cases}
\text{soglia} - \text{valore verificato} & \text{massimo (at\_most, less\_than)} \\
\text{valore verificato} - \text{soglia} & \text{minimo (at\_least, greater\_than)}
\end{cases}
\qquad
\text{headroom relativo} = \frac{\text{headroom}}{\text{soglia}}
$$

L'headroom è positivo dal lato in cui il covenant è rispettato e negativo dal lato della
violazione, qualunque sia la direzione del covenant. Alla soglia è zero: un operatore inclusivo
(`at_most`, `at_least`) è rispettato e uno stretto (`less_than`, `greater_than`) è violato.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

ltm = "for the 12 months ending on the test date"
r = fm.ratios.covenants(
    covenants=[
        {
            "name": "net leverage",
            "test_date": "2026-12-31",
            "numerator": {"definition": "consolidated net financial debt", "amount": 1900},
            "denominator": {"definition": f"consolidated EBITDA {ltm}", "amount": 600},
            "operator": "at_most",
            "threshold": "3.5",
        },
        {
            "name": "interest cover",
            "test_date": "2026-12-31",
            "numerator": {"definition": f"consolidated EBITDA {ltm}", "amount": 600},
            "denominator": {"definition": f"net finance charges {ltm}", "amount": 175},
            "operator": "at_least",
            "threshold": "4",
        },
        {
            "name": "debt service cover",
            "test_date": "2026-12-31",
            "numerator": {"definition": f"cash flow available for debt service {ltm}", "amount": 700},
            "denominator": {"definition": f"debt service {ltm}", "amount": 0},
            "operator": "at_least",
            "threshold": "1.2",
        },
    ]
)
leverage, cover, dscr = r.covenants
assert leverage["outcome"] == "passed"  # 1900 / 600 = 3.1666... <= 3.5
assert abs(leverage["headroom"] - Decimal("0.3333333333")) < Decimal("1e-10")
assert cover["outcome"] == "breached"  # 600 / 175 = 3.4285... < 4
assert cover["headroom"] < 0  # negativo: dal lato della violazione
assert dscr["outcome"] == "not_tested"  # denominatore nullo: mai una violazione
assert dscr["not_tested_reason"].startswith("denominator is zero")
assert r.summary == {"passed": 1, "breached": 1, "not_tested": 1}
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `covenants` | Elenco dei covenant, almeno uno; ciascuno è verificato da solo e i risultati mantengono l'ordine | obbligatorio |
| `covenants[].name` | Nome del covenant come nel contratto di finanziamento, non vuoto | obbligatorio |
| `covenants[].test_date` | Data di verifica, `YYYY-MM-DD`; gli importi si riferiscono al periodo che termina in quella data | obbligatorio |
| `covenants[].numerator` | `definition` (le parole del contratto, non vuote) e `amount` (unità monetarie) | obbligatorio |
| `covenants[].denominator` | Come `numerator`; zero significa non verificato, un valore negativo è rifiutato | obbligatorio |
| `covenants[].operator` | `at_most` (≤), `less_than` (<), `at_least` (≥) o `greater_than` (>) | obbligatorio, nessun default |
| `covenants[].threshold` | Nell'unità dell'indice: un multiplo (`3.5`) o una frazione decimale (`0.30` per 30%) | obbligatorio |

## Output

| Campo | Significato |
| --- | --- |
| `covenants` | Un risultato per covenant, nell'ordine dell'input |
| `covenants[].name`, `test_date`, `operator`, `threshold` | Riportati dall'input |
| `covenants[].outcome` | `passed`, `breached` o `not_tested` |
| `covenants[].value` | Valore verificato, numeratore / denominatore, esatto; null se non verificato |
| `covenants[].headroom` | Distanza dalla soglia nell'unità dell'indice, positiva dal lato del rispetto; null se non verificato |
| `covenants[].headroom_relative` | `{"value": ..., "not_defined": ...}`: headroom / soglia, frazione decimale, con lo stesso segno dell'headroom |
| `covenants[].not_tested_reason` | Perché il covenant non è stato verificato; null se verificato |
| `summary` | `passed`, `breached`, `not_tested`: quanti covenant hanno ciascun esito |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Definizioni | numeratore e denominatore come definiti dal contratto di finanziamento, forniti come importi; nessun indice standard li sostituisce | fissa |
| Data di verifica | gli importi si riferiscono al periodo che termina alla data di verifica | fissa |
| Operatore | `at_most`, `less_than`, `at_least`, `greater_than`, indicato per ogni covenant | obbligatorio |
| Confronto | esatto, senza arrotondare il valore verificato | fissa |
| Segno dell'headroom | positivo dal lato del rispetto, negativo dal lato della violazione | fissa |
| Headroom relativo | headroom / soglia; non definito quando la soglia è nulla o negativa, o il covenant non è verificato | fissa |
| Non verificato | il denominatore è nullo: il covenant non è verificato, con il motivo, e non è mai una violazione | fissa |

Ogni covenant aggiunge la propria regola di verifica ("net leverage (2026-12-31): passes when
the tested value ≤ 3.5 (maximum, inclusive)") e, se non è verificato, il motivo. Un numeratore
negativo è confrontato così come risulta dal calcolo: una posizione di cassa netta dà una leva
negativa, che rispetta un massimo; un EBITDA negativo dà una copertura negativa, che viola un
minimo.

## Non ancora deciso

I casi seguenti dipendono dai termini di ciascun contratto di finanziamento e sono rimessi ai
maintainer prima che Fairmark li modelli. Fino ad allora il metodo li rifiuta o li lascia non
definiti invece di scegliere.

- Un denominatore negativo (per esempio debito netto su un EBITDA negativo) è rifiutato con
  `FM-E001`: alcuni contratti lo trattano come una violazione, altri diversamente.
- Il valore verificato non è arrotondato; un contratto che arrotonda l'indice prima del
  confronto non è modellato.
- L'headroom relativo su una soglia negativa non è definito.
- I covenant su un singolo importo (patrimonio netto minimo, investimenti massimi), l'equity
  cure e gli step-down (riduzione programmata della soglia) non sono modellati; uno step-down è
  una voce per ogni data di verifica con la propria soglia.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | `covenants` è vuoto; `name` o `definition` vuoti o composti solo da spazi; un denominatore negativo; un campo mancante o sconosciuto, o un `operator` sconosciuto |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |
| `FM-E009` | Una `test_date` che non è una data `YYYY-MM-DD` valida |

## Precisione

Il valore verificato è una divisione decimale; headroom e headroom relativo sono una
sottrazione e una divisione. I risultati sono esatti fino alle 28 cifre significative del tipo
decimale, e il confronto con la soglia usa quel valore esatto.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/ratios.covenants/`, i cui valori attesi
  provengono da `scripts/reference_ratios.py` (decimal di Python a 50 cifre): un insieme di
  covenant con una violazione e l'headroom degli altri, un denominatore nullo, entrambi gli
  operatori di ogni direzione alla soglia, numeratori negativi e soglie non positive.
