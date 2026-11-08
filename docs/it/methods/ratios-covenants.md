---
source: methods/ratios-covenants.md
source_hash: 16a839a4d6c3
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

## Verifica dei covenant e indici di analisi

La verifica di un covenant può dare un numero diverso dall'indice di analisi con lo stesso nome
in [`ratios.debt_service`](ratios-debt-service.md), ed entrambi sono corretti per il proprio
scopo. L'indice di analisi usa gli aggregati di bilancio secondo le convenzioni scelte
dall'analista (saldi, definizioni di EBITDA e oneri finanziari). Il covenant usa le definizioni
scritte nel contratto di finanziamento, che di norma rettificano l'EBITDA (componenti non
ricorrenti, effetto pro forma di acquisizioni e cessioni), includono o escludono voci di debito
(leasing, finanziamenti dei soci), limitano la cassa che può essere portata in riduzione e
fissano il periodo di riferimento (spesso i 12 mesi che terminano alla data di verifica). La
banca valuta il rispetto solo sulle definizioni del contratto, quindi la verifica del covenant
non sostituisce mai a esse l'indice di analisi.

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

L'esito è deciso in modo esatto; valore e headroom sono mostrati con le 28 cifre significative
del tipo decimale, non arrotondati ai decimali del contratto. La decisione è il segno di
numeratore − soglia × denominatore, calcolato in modo esatto in aritmetica intera (a quel
punto il denominatore è positivo), quindi un valore verificato che a 28 cifre si arrotonda alla
soglia è comunque classificato sul suo valore esatto.

Nota: `headroom_relative` è headroom / soglia. Per un covenant di massimo con valore verificato
positivo è anche la frazione di cui può scendere il denominatore, a numeratore invariato, prima
della violazione. Per un covenant di minimo la frazione di cui può scendere il numeratore, a
denominatore invariato, è headroom / valore (per esempio una copertura minima degli oneri
finanziari pari a 4 verificata a 5: headroom 1, il numeratore può scendere di 1 / 5 = 20%,
mentre headroom / soglia è 25%). Fairmark non restituisce headroom / valore come campo a sé:
si ricava da `headroom` e `value`.

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
assert dscr["not_tested_code"] == "zero_denominator"
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
| `covenants[].outcome` | `passed`, `breached` o `not_tested`, deciso in modo esatto |
| `covenants[].value` | Valore verificato, numeratore / denominatore, con 28 cifre significative; null se non verificato |
| `covenants[].headroom` | Distanza dalla soglia nell'unità dell'indice, positiva dal lato del rispetto, con 28 cifre significative; null se non verificato |
| `covenants[].headroom_relative` | `{"value": ..., "not_defined": ...}`: headroom / soglia, frazione decimale, con lo stesso segno dell'headroom |
| `covenants[].not_tested_code` | `zero_denominator` o `overflow`; null se verificato |
| `covenants[].not_tested_reason` | Perché il covenant non è stato verificato, a parole; null se verificato |
| `summary` | `passed`, `breached`, `not_tested`: quanti covenant hanno ciascun esito |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Definizioni | numeratore e denominatore come definiti dal contratto di finanziamento, forniti come importi; nessun indice standard li sostituisce | fissa |
| Data di verifica | gli importi si riferiscono al periodo che termina alla data di verifica | fissa |
| Operatore | `at_most`, `less_than`, `at_least`, `greater_than`, indicato per ogni covenant | obbligatorio |
| Precisione | l'esito è deciso in modo esatto; valore e headroom sono mostrati con le 28 cifre significative del tipo decimale, non arrotondati ai decimali del contratto | fissa |
| Numeratore negativo | confrontato così come risulta dal calcolo | fissa |
| Segno dell'headroom | positivo dal lato del rispetto, negativo dal lato della violazione | fissa |
| Headroom relativo | headroom / soglia; non definito quando la soglia è nulla o negativa, o il covenant non è verificato | fissa |
| Non verificato | il denominatore è nullo (`zero_denominator`), oppure valore o headroom superano l'intervallo del tipo decimale (`overflow`): il covenant non è verificato, con il motivo, non è mai una violazione, e gli altri covenant sono verificati | fissa |

I covenant sono numerati da 1 nella tracciabilità del calcolo. Ogni covenant aggiunge la propria
regola di verifica ("covenant 1, net leverage (2026-12-31): passes when the tested value ≤ 3.5
(maximum, inclusive)"), un passo con l'esito ("breached (600 / 175 at_least 4 is false, decided
exactly)") e, se non è verificato, il motivo; anche i tre conteggi del riepilogo sono passi. Un
numeratore negativo è confrontato così come risulta dal calcolo: una posizione di cassa netta dà una leva
negativa, che rispetta un massimo; un EBITDA negativo dà una copertura negativa, che viola un
minimo.

## Non ancora deciso

I casi seguenti dipendono dai termini di ciascun contratto di finanziamento e sono rimessi ai
maintainer prima che Fairmark li modelli. Fino ad allora il metodo li rifiuta o li lascia non
definiti invece di scegliere.

- Un denominatore negativo (per esempio debito netto su un EBITDA negativo) è rifiutato con
  `FM-E001`: alcuni contratti lo trattano come una violazione, altri diversamente.
- Un contratto che arrotonda l'indice a un numero di decimali prima del confronto non è
  modellato: l'esito è deciso sul valore esatto.
- L'headroom relativo su una soglia negativa non è definito.
- I covenant su un singolo importo (patrimonio netto minimo, investimenti massimi), l'equity
  cure e gli step-down (riduzione programmata della soglia) non sono modellati; uno step-down è
  una voce per ogni data di verifica con la propria soglia.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | `covenants` è vuoto; `name` o `definition` vuoti o composti solo da spazi; un denominatore negativo; un campo mancante o sconosciuto, o un `operator` sconosciuto |
| `FM-E009` | Una `test_date` che non è una data `YYYY-MM-DD` valida |

## Precisione

L'esito è esatto: confronta numeratore e soglia × denominatore come numeri interi, senza
arrotondamenti. Il valore verificato è una divisione decimale, l'headroom una sottrazione e
l'headroom relativo una divisione, ciascuno con le 28 cifre significative del tipo decimale.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/ratios.covenants/`, i cui valori attesi
  provengono da `scripts/reference_ratios.py` (decimal di Python a 50 cifre): un insieme di
  covenant con una violazione e l'headroom degli altri, un denominatore nullo, entrambi gli
  operatori di ogni direzione alla soglia, numeratori negativi e soglie non positive; un caso
  costruito per l'overflow.
- Test unitari su valori che a 28 cifre si arrotondano alla soglia, per esempio
  3.0000000000000000000000000001 / 3 verificato con massimo 1, che risulta violato.
