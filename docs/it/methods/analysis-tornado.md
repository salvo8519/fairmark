---
source: methods/analysis-tornado.md
source_hash: e57262df337f
---
# Tornado (`analysis.tornado`)

Ordina gli input di qualsiasi metodo registrato in base a quanto muovono un output decimale.
Ogni input varia da solo tra un valore basso e un valore alto espliciti, gli altri restano al
valore del caso base, e gli input sono ordinati per escursione assoluta dell'output: sono i dati
alla base di un grafico a tornado. Domanda tipica: quale ipotesi del DCF incide di più
sull'equity value.

Il tornado è un metodo di analisi: valuta un altro metodo tramite il registro (vedi
[Metodi di analisi](../concepts/analysis.md)). Non può valutare un altro metodo di analisi.

## Formula

Per ogni variabile $x_k$ con valori $\text{low}_k < \text{high}_k$, con gli altri input al
valore del caso base:

$$
\text{swing}_k = y(x_k = \text{high}_k) - y(x_k = \text{low}_k),
\qquad
\text{bars ranked by } \lvert \text{swing}_k \rvert \text{, largest first}
$$

L'escursione è nell'unità di misura dell'output e mantiene il segno: è negativa quando l'output
diminuisce all'aumentare dell'input. A parità di escursione assoluta le variabili mantengono
l'ordine in cui sono state indicate.

## Esempio

Il DCF a cinque anni di [`dcf.fcff`](dcf-fcff.md): flussi di cassa operativi disponibili di 100,
110, 120, 130 e 140 unità monetarie, tasso di attualizzazione del 9%, tasso di crescita di lungo
periodo del 2% e posizione finanziaria netta di 300. Quattro determinanti dell'equity value:
posizione finanziaria netta da 250 a 350, flusso del primo anno da 90 a 110, tasso di crescita
di lungo periodo dall'1% al 3% e tasso di attualizzazione dall'8% al 10%.

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
t = fm.analysis.tornado(
    "dcf.fcff",
    base,
    output="/equity_value",
    variables={
        "/net_financial_position/gross_debt": ("250", "350"),
        "/stages/0/flows/0": ("90", "110"),
        "/terminal/tv.gordon/growth": ("0.01", "0.03"),
        "/wacc": ("0.08", "0.10"),
    },
)
print(t.base)  # 1485.9356357015730744749501235
for bar in t.ranking:
    print(bar["variable"], bar["swing"])
# /wacc -536.7420151241019796250302455
# /terminal/tv.gordon/growth 413.2480397880313369436240625
# /net_financial_position/gross_debt -100.0000000000000000000000000
# /stages/0/flows/0 18.3486238532110091743119266

assert [bar["variable"] for bar in t.ranking] == [
    "/wacc", "/terminal/tv.gordon/growth", "/net_financial_position/gross_debt", "/stages/0/flows/0",
]
# Escursioni attese da un calcolo indipendente a 50 cifre (scripts/reference_analysis.py).
expected = ["-536.74201512410197962503024533", "413.24803978803133694362406247",
            "-100", "18.348623853211009174311926606"]
for bar, swing in zip(t.ranking, expected):
    assert abs(bar["swing"] - Decimal(swing)) < Decimal("0.000000001")
    assert bar["abs_swing"] == abs(bar["swing"])

# La stessa richiesta tramite il punto di accesso generico, come la invia un agente o la CLI.
same = fm.run("analysis.tornado", {
    "method": "dcf.fcff",
    "base_input": base,
    "output": "/equity_value",
    "variables": [
        {"variable": "/net_financial_position/gross_debt", "low": "250", "high": "350"},
        {"variable": "/stages/0/flows/0", "low": "90", "high": "110"},
        {"variable": "/terminal/tv.gordon/growth", "low": "0.01", "high": "0.03"},
        {"variable": "/wacc", "low": "0.08", "high": "0.10"},
    ],
})
assert same.ranking == t.ranking
```

Il tasso di attualizzazione è la variabile dominante: due punti percentuali muovono l'equity
value di circa 537 unità monetarie, più di cinque volte l'effetto di 100 di posizione
finanziaria netta. I passaggi del risultato elencano il caso base, poi, per ogni variabile
nell'ordine indicato, l'output al valore basso, l'output al valore alto e l'escursione.

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `method` | Identificativo del metodo da valutare, ad esempio `dcf.fcff`; i metodi di analisi non sono ammessi | obbligatorio |
| `base_input` | Input completo di quel metodo, esattamente come il metodo lo accetta: il caso base; ogni variabile cambia da sola, le altre mantengono il valore del caso base | obbligatorio |
| `output` | JSON Pointer RFC 6901 all'output decimale da ordinare, nel `value` del risultato interno, ad esempio `/equity_value` | obbligatorio |
| `variables` | Gli input da far variare; almeno uno; i puntatori non si ripetono | obbligatorio |
| `variables[].variable` | JSON Pointer RFC 6901 a un input decimale in `base_input`, ad esempio `/discount_rate` | obbligatorio |
| `variables[].low` | Valore basso dell'input, nella sua unità di misura, inferiore a `high` | obbligatorio |
| `variables[].high` | Valore alto dell'input, nella sua unità di misura | obbligatorio |

I valori basso e alto sono valori espliciti, non variazioni percentuali: "±10% su un tasso del
9%" potrebbe significare dall'8,1% al 9,9% oppure dall'8% al 10%, quindi è il chiamante a
indicare i valori. In Python, `variables` associa a ogni puntatore la coppia `(low, high)`;
l'ordine è mantenuto.

## Output

| Campo | Significato |
| --- | --- |
| `inner_method` | `id` e `version` del metodo valutato, nella versione eseguita da questo motore |
| `output` | Puntatore dell'output ordinato |
| `base` | Output per `base_input` senza modifiche, nell'unità di misura dell'output |
| `ranking` | Le barre per `abs_swing` decrescente; a parità di escursione è mantenuto l'ordine di `variables` |
| `ranking[].variable`, `low`, `high` | L'input fatto variare e i suoi due valori, come indicati |
| `ranking[].output_low`, `output_high` | L'output con l'input a `low` e a `high`, nell'unità di misura dell'output |
| `ranking[].swing` | `output_high − output_low`, nell'unità di misura dell'output; negativo quando l'output diminuisce all'aumentare dell'input |
| `ranking[].abs_swing` | $\lvert$`swing`$\rvert$: la lunghezza della barra e il criterio di ordinamento |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Valore | Default |
| --- | --- | --- |
| Metodo interno | identificativo e versione, valutato tramite il registro | obbligatorio |
| Stato del metodo interno | se il metodo interno è in anteprima, "Evaluated method <id> is preview"; lo stato è anche in `meta.evaluated_method_status` | riportato |
| Output | puntatore misurato dalle barre | obbligatorio |
| Variazione | una variabile alla volta; gli altri input mantengono il valore del caso base | fisso |
| Valori di variazione | valori basso e alto espliciti; nessuna variazione percentuale | obbligatorio |
| Ordine di valutazione | caso base, poi ogni variabile nell'ordine indicato, il valore basso prima di quello alto | fisso |
| Ordinamento | per escursione assoluta, dalla maggiore; a parità di escursione è mantenuto l'ordine indicato | fisso |
| Valutazioni in errore | qualsiasi valutazione rifiutata dal metodo interno interrompe la richiesta con il relativo errore | fisso |
| Codifica delle variabili | scritte nell'input del metodo interno come stringhe decimali | fissa |

L'ultima convenzione riportata è il numero di valutazioni del metodo interno: due per variabile
più il caso base.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | `variables` è vuoto, una variabile si ripete, low ≥ high, oppure un campo è mancante o sconosciuto |
| `FM-E007` | `method` non corrisponde ad alcun metodo registrato |
| `FM-E015` | Una variabile non si risolve in un decimale in `base_input`, oppure `output` non si risolve in un decimale nel risultato del caso base; il messaggio indica il puntatore |
| `FM-E016` | Due valutazioni per variabile più il caso base superano 10.000 oppure valutazioni × valori in `base_input` superano 1.000.000; restituito prima della prima valutazione |
| `FM-E017` | `method` indica un metodo di analisi |

I puntatori delle variabili, i valori basso e alto, la sintassi di `output` e `base_input` sono
verificati prima di qualsiasi valutazione, `base_input` con la sola validazione del metodo interno (ogni controllo del suo input,
nessun calcolo, i suoi errori di input riportati sotto `/base_input`); `output` è risolto sul caso base. A differenza delle tabelle di
sensitività e dell'analisi di scenario, un errore del metodo interno in qualsiasi valutazione,
compreso il caso base, interrompe la richiesta ed è restituito invariato con il proprio codice:
un ordinamento con una barra mancante non è definito. Ad esempio, un valore alto del tasso di
crescita di lungo periodo pari o superiore al tasso di attualizzazione di `dcf.fcff` restituisce
`FM-E004`; scegli valori basso e alto per i quali il metodo interno sia valido.

## Precisione

Gli output sono precisi quanto il metodo interno (vedi la relativa pagina); le escursioni sono
differenze esatte degli output riportati. I casi di riferimento sono verificati entro
$10^{-9}$ nell'unità di misura dell'output; i casi di parità sono verificati in modo esatto.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/analysis.tornado/`: quattro determinanti
  del DCF, i cui ordinamento ed escursioni attesi provengono da `scripts/reference_analysis.py`
  (`dcf.fcff` ricostruito dalla formula documentata con il modulo `decimal` di Python a 50
  cifre; non condivide codice con il motore), e un caso di parità costruito su `tvm.npv` con
  tasso 0, in cui una variazione di ±1 su uno qualsiasi di due flussi di cassa muove il VAN di 2
  e l'ordine di dichiarazione è mantenuto.
