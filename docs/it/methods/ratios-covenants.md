---
source: methods/ratios-covenants.md
source_hash: 752c2aca5581
---
# Verifica dei covenant (`ratios.covenants`)

Risponde alla domanda "la società rispetta i propri covenant?". Verifica un elenco di covenant
finanziari, ciascuno sul numeratore e sul denominatore definiti dal contratto di finanziamento,
rispetto a una soglia a una data di verifica, e restituisce per ogni covenant un esito esplicito
(`passed`, rispettato; `breached`, violato; `not_tested`, non verificato, con un codice e il
motivo), il valore verificato e l'headroom (margine rispetto alla soglia).

Il metodo non calcola gli indici standard degli altri gruppi: un covenant è verificato sulle
definizioni del contratto (per esempio "EBITDA consolidato dei 12 mesi che terminano alla data
di verifica, prima delle componenti non ricorrenti"), che spesso differiscono dai dati di
bilancio. L'analista calcola gli importi secondo quelle definizioni e li fornisce con le parole
del contratto; Fairmark riporta entrambi nella tracciabilità del calcolo (audit trail).

Fairmark non decide mai ciò che decide il contratto di finanziamento. Dove i contratti
differiscono, la clausola è un input esplicito senza default: l'esito quando il denominatore è
nullo o negativo (`if_denominator_not_positive`) e l'arrotondamento del valore verificato
(`rounding`). Ogni esito risale così a una clausola del contratto.

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
\quad\text{(arrotondato come stabilisce il contratto, se è indicato l'arrotondamento)}
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

**Precisione.** L'esito è deciso in modo esatto. Senza `rounding` è il segno di
numeratore − soglia × denominatore, calcolato in modo esatto in aritmetica intera (a quel punto
il denominatore è positivo), quindi un valore verificato che a 28 cifre si arrotonda alla soglia
è comunque classificato sul suo valore esatto; valore e headroom sono mostrati con le 28 cifre
significative del tipo decimale, non arrotondati ai decimali del contratto. Con `rounding` il
valore verificato è arrotondato come stabilisce il contratto, a partire dal quoziente esatto
(senza arrotondamenti intermedi), e l'esito è il confronto esatto di quel valore arrotondato
con la soglia; `value` è il valore arrotondato e `value_unrounded` il valore prima
dell'arrotondamento. Quando l'headroom mostrato si arrotonda a 0 con 28 cifre significative (o
mostra il segno opposto) mentre il confronto esatto decide diversamente, una convenzione e una
nota nella tracciabilità del calcolo lo segnalano: "displayed headroom rounds to 0 at 28
significant digits; the exact comparison decides breached".

**Denominatore nullo o negativo.** Il valore verificato non è calcolato: valore e headroom sono
null e l'esito è quello indicato da `if_denominator_not_positive` (`breached`, `passed` o
`not_tested`), e `outcome_basis` è `if_denominator_not_positive`, così un agente distingue un
esito fissato dalla regola del contratto da uno deciso su un valore verificato
(`tested_value`). Con `not_tested` il codice è `denominator_not_positive` e il motivo indica se il
denominatore è nullo o negativo.

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
            "if_denominator_not_positive": "breached",
        },
        {
            "name": "interest cover",
            "test_date": "2026-12-31",
            "numerator": {"definition": f"consolidated EBITDA {ltm}", "amount": 600},
            "denominator": {"definition": f"net finance charges {ltm}", "amount": 175},
            "operator": "at_least",
            "threshold": "4",
            "if_denominator_not_positive": "passed",
        },
        {
            "name": "debt service cover",
            "test_date": "2026-12-31",
            "numerator": {"definition": f"cash flow available for debt service {ltm}", "amount": 700},
            "denominator": {"definition": f"debt service {ltm}", "amount": 0},
            "operator": "at_least",
            "threshold": "1.2",
            "if_denominator_not_positive": "not_tested",
        },
        {
            "name": "senior leverage",
            "test_date": "2026-12-31",
            "numerator": {"definition": "senior net financial debt", "amount": 3504},
            "denominator": {"definition": f"consolidated EBITDA {ltm}", "amount": 1000},
            "operator": "at_most",
            "threshold": "3.50",
            "if_denominator_not_positive": "breached",
            "rounding": {"decimal_places": 2, "mode": "half_up"},
        },
    ]
)
leverage, cover, dscr, senior = r.covenants
assert leverage["outcome"] == "passed"  # 1900 / 600 = 3.1666... <= 3.5
assert abs(leverage["headroom"] - Decimal("0.3333333333")) < Decimal("1e-10")
assert cover["outcome"] == "breached"  # 600 / 175 = 3.4285... < 4
assert cover["headroom"] < 0  # negativo: dal lato della violazione
assert dscr["outcome"] == "not_tested"  # servizio del debito nullo, come stabilisce il contratto
assert dscr["not_tested_code"] == "denominator_not_positive"
assert dscr["outcome_basis"] == "if_denominator_not_positive"
assert leverage["outcome_basis"] == "tested_value"
assert dscr["not_tested_reason"].startswith("denominator is zero")
# 3.504 si arrotonda a 3.50 secondo il contratto, che rispetta il massimo di 3.50
assert senior["outcome"] == "passed"
assert senior["value_unrounded"] == Decimal("3.504")
assert r.summary == {"passed": 2, "breached": 1, "not_tested": 1}
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `covenants` | Elenco dei covenant, almeno uno; ciascuno è verificato da solo e i risultati mantengono l'ordine | obbligatorio |
| `covenants[].name` | Nome del covenant come nel contratto di finanziamento, non vuoto | obbligatorio |
| `covenants[].test_date` | Data di verifica, `YYYY-MM-DD`; gli importi si riferiscono al periodo che termina in quella data | obbligatorio |
| `covenants[].numerator` | `definition` (le parole del contratto, non vuote) e `amount` (unità monetarie) | `definition` obbligatorio; `amount` facoltativo (vedi Input parziale) |
| `covenants[].denominator` | Come `numerator` | `definition` obbligatorio; `amount` facoltativo |
| `covenants[].operator` | `at_most` (≤), `less_than` (<), `at_least` (≥) o `greater_than` (>) | obbligatorio, nessun default |
| `covenants[].threshold` | Nell'unità dell'indice: un multiplo (`3.5`) o una frazione decimale (`0.30` per 30%) | obbligatorio |
| `covenants[].if_denominator_not_positive` | Esito quando il denominatore è nullo o negativo, come stabilisce il contratto: `breached`, `passed` o `not_tested` | obbligatorio, nessun default |
| `covenants[].rounding` | Arrotondamento contrattuale del valore verificato prima del confronto: `decimal_places` (da 0 a 6) e `mode` (`half_up`: le metà si arrotondano allontanandosi dallo zero; `half_even`: le metà si arrotondano alla cifra pari), entrambi obbligatori | facoltativo: nessun arrotondamento |

## Output

| Campo | Significato |
| --- | --- |
| `covenants` | Un risultato per covenant, nell'ordine dell'input |
| `covenants[].name`, `test_date`, `operator`, `threshold` | Riportati dall'input |
| `covenants[].outcome` | `passed`, `breached` o `not_tested`, deciso in modo esatto |
| `covenants[].outcome_basis` | Che cosa ha deciso l'esito: `tested_value`, oppure `if_denominator_not_positive` quando il denominatore è nullo o negativo e si applica la regola indicata in input; null se il covenant non è verificato per un overflow |
| `covenants[].value` | Il valore confrontato con la soglia: numeratore / denominatore con 28 cifre significative, oppure arrotondato come indica `rounding`; null se il denominatore non è positivo o il covenant non è verificato |
| `covenants[].value_unrounded` | Con `rounding`: numeratore / denominatore prima dell'arrotondamento; null altrimenti |
| `covenants[].headroom` | Distanza di `value` dalla soglia nell'unità dell'indice, positiva dal lato del rispetto; null se `value` è null |
| `covenants[].headroom_relative` | `{"value": ..., "not_defined": ...}`: headroom / soglia, frazione decimale, con lo stesso segno dell'headroom |
| `covenants[].not_tested_code` | `denominator_not_positive`, `overflow` o `missing_input`; null se l'esito è `passed` o `breached` |
| `covenants[].not_tested_reason` | Perché il covenant non è stato verificato, a parole; null se l'esito è `passed` o `breached` |
| `summary` | `passed`, `breached`, `not_tested`: quanti covenant hanno ciascun esito |
| `not_computed` | Per ogni covenant non verificato perché manca un importo, i suoi `value`, `headroom` e `headroom_relative`: `{"output", "missing", "reason"}`, con `output` il JSON pointer dell'output nel valore del risultato (`/covenants/0/value`) e `missing` i JSON pointer degli importi che gli mancano; vuoto altrimenti |

## Input parziale

Gli importi sono facoltativi (versione 2, SPEC-0007 R11). Un covenant di cui non è fornito
l'importo del denominatore, oppure con denominatore positivo e senza l'importo del numeratore,
non è verificato con `not_tested_code` `missing_input`, mai una violazione, e i suoi output sono
elencati in `not_computed`. Un denominatore fornito nullo o negativo dà l'esito di
`if_denominator_not_positive` qualunque sia il numeratore, anche quando l'importo del numeratore
non è fornito, perché il contratto decide quel caso senza di esso: il covenant è deciso, non
elencato. Un importo mancante non è mai considerato zero. La soglia minima: almeno un covenant
deve poter essere verificato o deciso dalla sua regola; altrimenti l'input è rifiutato con
`FM-E027`, il cui messaggio elenca gli importi che la superano.

```python
r = fm.ratios.covenants(covenants=[
    {"name": "net leverage", "test_date": "2026-12-31",
     "numerator": {"definition": "net debt"},
     "denominator": {"definition": "EBITDA, last 12 months", "amount": 1000},
     "operator": "at_most", "threshold": "3.5",
     "if_denominator_not_positive": "breached"},
    {"name": "interest cover", "test_date": "2026-12-31",
     "numerator": {"definition": "EBITDA, last 12 months", "amount": 1000},
     "denominator": {"definition": "net finance charges", "amount": 200},
     "operator": "at_least", "threshold": "4",
     "if_denominator_not_positive": "breached"}])
c = r.covenants[0]
assert (c["outcome"], c["not_tested_code"]) == ("not_tested", "missing_input")
assert r.not_computed[0]["missing"] == ["/covenants/0/numerator/amount"]
assert r.covenants[1]["outcome"] == "passed"  # 1000 / 200 = 5, at least 4
# Con il solo primo covenant l'input sarebbe sotto la soglia minima (FM-E027).
```

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Definizioni | numeratore e denominatore come definiti dal contratto di finanziamento, forniti come importi; nessun indice standard li sostituisce | fissa |
| Data di verifica | gli importi si riferiscono al periodo che termina alla data di verifica | fissa |
| Operatore | `at_most`, `less_than`, `at_least`, `greater_than`, indicato per ogni covenant | obbligatorio |
| Denominatore nullo o negativo | `breached`, `passed` o `not_tested`, indicato per ogni covenant; valore e headroom null | obbligatorio |
| Arrotondamento | nessuno, oppure `decimal_places` da 0 a 6 con `half_up` o `half_even`, indicato per ogni covenant | nessuno |
| Precisione | l'esito è deciso in modo esatto: sul valore verificato esatto, oppure sul valore arrotondato come stabilisce il contratto quando è indicato l'arrotondamento; valore e headroom sono mostrati con le 28 cifre significative del tipo decimale, e sono arrotondati ai decimali del contratto solo quando è indicato l'arrotondamento | fissa |
| Numeratore negativo | confrontato così come risulta dal calcolo | fissa |
| Segno dell'headroom | positivo dal lato del rispetto, negativo dal lato della violazione | fissa |
| Headroom relativo | headroom / soglia; non definito quando la soglia è nulla o negativa, o quando non c'è un valore verificato | fissa |
| Overflow | il valore, il valore arrotondato o l'headroom superano l'intervallo del tipo decimale: non verificato (`overflow`), mai una violazione, e gli altri covenant sono verificati | fissa |
| Importo mancante | non verificato (`missing_input`), mai una violazione, salvo che un denominatore fornito nullo o negativo decida con la sua regola; elencato in `not_computed` | fissa |

I covenant sono numerati da 1 nella tracciabilità del calcolo. Ogni covenant aggiunge la propria
regola di verifica ("covenant 1, net leverage (2026-12-31): passes when the tested value ≤ 3.5
(maximum, inclusive)"), la propria regola per un denominatore non positivo e il proprio
arrotondamento, entrambi indicati come provenienti dall'input, un passo con l'esito ("breached
(600 / 175 at_least 4 is false, decided exactly)"), il valore non arrotondato e arrotondato
quando è indicato `rounding` e, se il covenant non è verificato, il motivo; anche i tre
conteggi del riepilogo sono passi. Un numeratore negativo è confrontato così come risulta dal
calcolo: una posizione di cassa netta dà una leva negativa, che rispetta un massimo; un EBITDA
negativo dà una copertura negativa, che viola un minimo.

## Non coperto

- I covenant su un singolo importo (patrimonio netto minimo, investimenti massimi) non hanno
  denominatore e non sono verificati da questo metodo; si veda la
  [roadmap](https://salvo8519.github.io/fairmark/project/roadmap/) (in inglese).
- L'equity cure non è modellata nella versione 1.
- Gli step-down (riduzione programmata della soglia) si esprimono con un covenant per ogni data
  di verifica, ciascuno con la propria soglia.
- L'headroom relativo su una soglia negativa non è definito.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | `covenants` è vuoto; `name` o `definition` vuoti o composti solo da spazi; `rounding.decimal_places` superiore a 6; un campo mancante o sconosciuto (compresi `operator`, `if_denominator_not_positive`, o `rounding.mode` quando è indicato `rounding`), o un valore sconosciuto |
| `FM-E027` | Sotto la soglia minima di input: nessun covenant può essere verificato o deciso dalla sua regola, perché a ciascuno manca un importo; il messaggio elenca gli insiemi alternativi di importi (ne basta uno) |
| `FM-E009` | Una `test_date` che non è una data `YYYY-MM-DD` valida |

## Precisione

L'esito è esatto: senza arrotondamento confronta numeratore e soglia × denominatore come numeri
interi; con l'arrotondamento il valore arrotondato è calcolato in modo esatto dal quoziente dei
due importi e confrontato con la soglia. Il valore verificato è una divisione decimale,
l'headroom una sottrazione e l'headroom relativo una divisione, ciascuno con le 28 cifre
significative del tipo decimale.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/ratios.covenants/`, i cui valori attesi
  provengono da `scripts/reference_ratios.py` (frazioni di Python per l'esito e
  l'arrotondamento, decimal a 50 cifre per i valori stampati): un insieme di covenant con una
  violazione e l'headroom degli altri, un denominatore nullo e uno negativo con ciascun valore
  di `if_denominator_not_positive`, 3.504 verificato con massimo 3.50 con e senza
  arrotondamento a 2 decimali, `half_up` e `half_even` su una metà, entrambi gli operatori di
  ogni direzione alla soglia, numeratori negativi e soglie non positive; un caso costruito per
  l'overflow.
- Un caso di riferimento con importi mancanti (`missing-amounts`) e un confronto casuale con seme
  fisso su input parziali, rispetto a `scripts/reference_ratios_partial.py`.
- Test unitari su valori che a 28 cifre si arrotondano alla soglia, per esempio
  3.0000000000000000000000000001 / 3 verificato con massimo 1, che risulta violato, e su un
  quoziente il cui arrotondamento a 28 cifre porterebbe a un arrotondamento errato a 2 decimali.
