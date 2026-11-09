---
source: methods/analysis-scenarios.md
source_hash: 570bdc7ca464
---
# Analisi di scenario (`analysis.scenarios`)

Riporta uno o più output decimali di qualsiasi metodo registrato in scenari nominati, ciascuno
definito da un insieme di modifiche degli input rispetto al caso base, con lo scostamento e lo
scostamento relativo di ogni output. Domanda tipica: l'enterprise value e l'equity value di un
DCF in uno scenario sfavorevole e in uno favorevole, e di quanto ciascuno si discosta dal caso
base. Uno scenario che il metodo interno rifiuta è registrato con il proprio errore e gli altri
scenari sono comunque restituiti.

L'analisi di scenario è un metodo di analisi: valuta un altro metodo tramite il registro (vedi
[Metodi di analisi](../concepts/analysis.md)). Non può valutare un altro metodo di analisi.

## Formula

Per ogni scenario $s$, con l'input base modificato dalle sostituzioni di $s$, e per ogni output
$y$:

$$
\Delta_s = y_s - y_{base},
\qquad
\delta_s = \frac{\Delta_s}{y_{base}} \quad (y_{base} \neq 0)
$$

$\Delta_s$ è nell'unità di misura dell'output; $\delta_s$ è una frazione decimale (`-0.2`
corrisponde a un calo del 20%) ed è `null` quando $y_{base}$ è zero. Gli input che uno scenario
non modifica mantengono il valore del caso base.

## Esempio

Il DCF a cinque anni di [`dcf.fcff`](dcf-fcff.md): flussi di cassa operativi disponibili di 100,
110, 120, 130 e 140 unità monetarie, tasso di attualizzazione del 9%, tasso di crescita di lungo
periodo del 2% e posizione finanziaria netta di 300. Tre scenari: uno sfavorevole (tasso 10%,
crescita 1%, flusso del primo anno 90), uno favorevole (tasso 8%, crescita 2,5%) e uno con
crescita del 12%, che il DCF rifiuta.

```python
from decimal import Decimal

import fairmark as fm

base = {
    "stages": [{"periods": 5, "projection": "given", "flows": [100, 110, 120, 130, 140]}],
    "cash_flow_terms": "nominal",
    "discount_rate_terms": "nominal",
    "wacc": "0.09",
    "timing": "end_of_period",
    "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
    "net_financial_position": {"gross_debt": 300, "cash": 0},
}
r = fm.analysis.scenarios(
    "dcf.fcff",
    base,
    outputs=["/enterprise_value", "/equity_value"],
    cases={
        "downside": {"/wacc": "0.10", "/terminal/tv.gordon/growth": "0.01",
                     "/stages/0/flows/0": "90"},
        "upside": {"/wacc": "0.08", "/terminal/tv.gordon/growth": "0.025"},
        "impossible": {"/terminal/tv.gordon/growth": "0.12"},
    },
)
print(r.base[1]["value"])        # 1485.9356357015730744749501235, equity value del caso base
downside = r.scenarios[0]["value"][1]
print(downside["value"])         # 1114.1421730452534359371324060
print(downside["delta"])         # -371.7934626563196385378177175
print(downside["relative_delta"])  # -0.250208322435702416213594832, un calo di circa il 25%
print(r.scenarios[2]["error"]["code"])  # FM-E004: crescita 12% non inferiore al tasso 9%
print(r.failed_scenarios)        # 1

# Valori attesi da un calcolo indipendente a 50 cifre (scripts/reference_analysis.py).
assert abs(downside["value"] - Decimal("1114.1421730452534359371324059528")) < Decimal("1e-9")
upside = r.scenarios[1]["value"][0]
assert abs(upside["value"] - Decimal("2248.6986924572658454688633015108")) < Decimal("1e-9")
assert abs(upside["relative_delta"] - Decimal("0.259115192902181230325447937")) < Decimal("1e-9")
# Lo scostamento è esattamente il valore dello scenario meno il valore del caso base.
assert downside["delta"] == downside["value"] - r.base[1]["value"]

# La stessa richiesta tramite il punto di accesso generico, come la invia un agente o la CLI.
same = fm.run("analysis.scenarios", {
    "method": "dcf.fcff",
    "base_input": base,
    "outputs": ["/enterprise_value", "/equity_value"],
    "scenarios": [
        {"name": "downside", "overrides": [
            {"pointer": "/wacc", "value": "0.10"},
            {"pointer": "/terminal/tv.gordon/growth", "value": "0.01"},
            {"pointer": "/stages/0/flows/0", "value": "90"},
        ]},
        {"name": "upside", "overrides": [
            {"pointer": "/wacc", "value": "0.08"},
            {"pointer": "/terminal/tv.gordon/growth", "value": "0.025"},
        ]},
        {"name": "impossible", "overrides": [
            {"pointer": "/terminal/tv.gordon/growth", "value": "0.12"},
        ]},
    ],
})
assert same.scenarios == r.scenarios
```

Lo scostamento relativo non è definito quando l'output del caso base è zero. Al 10% il VAN di
−100 e 110 è esattamente zero, quindi uno scenario al 5% riporta lo scostamento e uno
scostamento relativo `null`:

```python
import fairmark as fm

npv = fm.analysis.scenarios(
    "tvm.npv",
    {"rate": "0.10", "cash_flows": [-100, 110]},
    outputs=["/npv"],
    cases={"lower rate": {"/rate": "0.05"}},
)
lower = npv.scenarios[0]["value"][0]
print(lower["delta"])            # 4.76190476190476190476190476
assert npv.base[0]["value"] == 0
assert lower["relative_delta"] is None
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `method` | Identificativo del metodo da valutare, ad esempio `dcf.fcff`; i metodi di analisi non sono ammessi | obbligatorio |
| `base_input` | Input completo di quel metodo, esattamente come il metodo lo accetta: il caso base | obbligatorio |
| `outputs` | JSON Pointer RFC 6901 agli output decimali da riportare, nel `value` del risultato interno, ad esempio `["/enterprise_value", "/equity_value"]`; almeno uno, senza ripetizioni | obbligatorio |
| `scenarios` | Gli scenari, valutati nell'ordine indicato; almeno uno | obbligatorio |
| `scenarios[].name` | Nome dello scenario, univoco e non vuoto, ad esempio `downside` | obbligatorio |
| `scenarios[].overrides` | Input modificati in questo scenario; i puntatori non si ripetono all'interno di uno scenario | obbligatorio |
| `overrides[].pointer` | JSON Pointer RFC 6901 a un input decimale in `base_input`, ad esempio `/terminal_growth` | obbligatorio |
| `overrides[].value` | Valore di quell'input in questo scenario, nella sua unità di misura (frazione decimale per un tasso) | obbligatorio |

In Python, `cases` associa a ogni nome di scenario un dizionario `{pointer: value}`; l'ordine è
mantenuto.

## Output

| Campo | Significato |
| --- | --- |
| `inner_method` | `id` e `version` del metodo valutato, nella versione eseguita da questo motore |
| `base` | Un `{output, value}` per ogni puntatore di output: gli output di `base_input` senza modifiche, nell'unità di misura dell'output |
| `scenarios` | Una voce per scenario, nell'ordine indicato: `{name, value: [...]}`, oppure `{name, error: {code, message}}` quando il metodo interno ha rifiutato lo scenario |
| `scenarios[].value[]` | Per ogni output: `output`, `value` (unità di misura dell'output), `delta` (`value − base`, unità di misura dell'output), `relative_delta` (`delta / base`, frazione decimale, `null` quando il caso base è zero) |
| `failed_scenarios` | Numero di scenari rifiutati dal metodo interno o i cui scostamenti superano l'intervallo dei decimali |

I passaggi elencano il valore del caso base di ogni output, poi un passaggio per ogni output di
ciascuno scenario valutato con successo, con le sostituzioni, il valore, lo scostamento e lo
scostamento relativo sostituiti.

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Valore | Default |
| --- | --- | --- |
| Metodo interno | identificativo e versione, valutato tramite il registro | obbligatorio |
| Stato del metodo interno | se il metodo interno è in anteprima, "Evaluated method <id> is preview"; lo stato è anche in `meta.evaluated_method_status` | riportato |
| Ordine di valutazione | caso base, poi gli scenari nell'ordine indicato | fisso |
| Input non modificati | mantengono il valore del caso base | fisso |
| Scostamento relativo quando l'output del caso base è zero | `null` | fisso |
| Scenari in errore | uno scenario rifiutato dal metodo interno, o i cui scostamenti superano l'intervallo dei decimali, contiene il codice e il messaggio di errore e gli altri proseguono; `FM-E012` (un difetto di Fairmark) interrompe la richiesta | fisso |
| Codifica delle sostituzioni | scritte nell'input del metodo interno come stringhe decimali | fissa |

L'ultima convenzione riportata è il numero di valutazioni del metodo interno e il numero di
scenari in errore. Una richiesta esegue una valutazione per scenario più il caso base.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | `outputs` o `scenarios` è vuoto, un output si ripete, il nome di uno scenario è vuoto, ha spazi iniziali o finali, o si ripete, un puntatore si ripete all'interno di uno scenario, oppure un campo è mancante o sconosciuto |
| `FM-E007` | `method` non corrisponde ad alcun metodo registrato |
| `FM-E015` | Il puntatore di una sostituzione non si risolve in un decimale in `base_input`, oppure il puntatore di un output non si risolve in un decimale nel risultato del caso base; il messaggio indica il puntatore |
| `FM-E016` | Il numero di scenari più uno supera 10.000 valutazioni oppure valutazioni × valori in `base_input` superano 1.000.000; restituito prima della prima valutazione |
| `FM-E017` | `method` indica un metodo di analisi |

I puntatori delle sostituzioni, la sintassi degli output e `base_input` sono verificati prima
di qualsiasi valutazione, `base_input` con la sola validazione del metodo interno (ogni controllo del suo input,
nessun calcolo, i suoi errori di input riportati sotto `/base_input`); gli output sono risolti sul caso base. Un errore del metodo interno sul caso base è
restituito invariato, con il proprio codice. Un errore del metodo interno in uno scenario è
registrato in quello scenario, salvo `FM-E012`, che interrompe la richiesta.

## Precisione

I valori sono precisi quanto il metodo interno (vedi la relativa pagina). Gli scostamenti sono
differenze esatte dei valori riportati; gli scostamenti relativi sono arrotondati alle 28 cifre
significative del tipo decimale. I casi di riferimento sono verificati entro $10^{-9}$
nell'unità di misura dell'output.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/analysis.scenarios/` (tre scenari DCF, di
  cui uno rifiutato, e un caso di VAN con caso base pari a zero), i cui valori attesi provengono
  da `scripts/reference_analysis.py`: `dcf.fcff` e `tvm.npv` ricostruiti dalle formule
  documentate con il modulo `decimal` di Python a 50 cifre, con scostamenti e scostamenti
  relativi calcolati a partire da essi. Non condivide codice con il motore.
- Un test richiede che ogni scostamento sia esattamente uguale al valore dello scenario meno il
  valore del caso base, e che uno scenario con una sostituzione non valida registri il proprio
  errore mentre gli altri sono restituiti.
