---
source: methods/analysis-sensitivity.md
source_hash: 492990d16930
---
# Tabella di sensitività (`analysis.sensitivity`)

Riporta un output decimale di qualsiasi metodo registrato al variare di un input (tabella a una
variabile) o di due input (tabella a due variabili): la tabella dati dei modelli costruiti su
foglio di calcolo. Domanda tipica: come varia l'enterprise value del DCF su una griglia di tassi
di attualizzazione e di tassi di crescita di lungo periodo. Una combinazione che il metodo
interno rifiuta, come una crescita non inferiore al tasso di attualizzazione, è registrata nella
propria cella e il resto della tabella è comunque restituito.

La tabella di sensitività è un metodo di analisi: valuta un altro metodo tramite il registro
(vedi [Metodi di analisi](../concepts/analysis.md)). Non può valutare un altro metodo di
analisi.

## Formula

Dati il metodo interno $M$, la variabile di riga $x_r$ con valori $r_1, \dots, r_m$, la
variabile di colonna facoltativa $x_c$ con valori $c_1, \dots, c_n$ e l'output $y$, ogni cella è

$$
\text{cell}_{ij} = y\big(M(\text{base input with } x_r = r_i \text{ and } x_c = c_j)\big),
\qquad i = 1, \dots, m, \quad j = 1, \dots, n
$$

Tutti gli altri input mantengono il valore del caso base. Una tabella a una variabile non ha
variabile di colonna e ha una sola colonna ($n = 1$). Il caso base, $y(M(\text{base input}))$,
è valutato per primo e riportato come `base`.

## Esempio

Il DCF a cinque anni di [`dcf.fcff`](dcf-fcff.md): flussi di cassa operativi disponibili di 100,
110, 120, 130 e 140 unità monetarie, tasso di attualizzazione del 9%, tasso di crescita di lungo
periodo del 2% e posizione finanziaria netta di 300. La tabella riporta l'enterprise value per
tassi di attualizzazione dell'8%, del 9% e del 10% (righe) e tassi di crescita di lungo periodo
dell'1%, del 2% e dell'8% (colonne).

```python
from decimal import Decimal

import fairmark as fm

base = {
    "free_cash_flows": [100, 110, 120, 130, 140],
    "discount_rate": "0.09",
    "terminal_growth": "0.02",
    "net_debt": 300,
}
table = fm.analysis.sensitivity(
    "dcf.fcff",
    base,
    output="/enterprise_value",
    rows=("/discount_rate", ["0.08", "0.09", "0.10"]),
    columns=("/terminal_growth", ["0.01", "0.02", "0.08"]),
)
print(table.base)                    # 1785.9356357015730744749501235
print(table.cells[0][0]["value"])    # 1847.7733182046548911356105383, tasso 8%, crescita 1%
print(table.cells[0][2]["error"])    # crescita 8% non inferiore al tasso 8%: FM-E004
print(table.failed_cells)            # 1

# Valori attesi da un calcolo indipendente a 50 cifre (scripts/reference_analysis.py).
expected = {
    (0, 0): "1847.7733182046548911356105381406",
    (1, 1): "1785.9356357015730744749501235001",
    (2, 2): "5141.8618946793251827060993101564",
}
for (i, j), value in expected.items():
    assert abs(table.cells[i][j]["value"] - Decimal(value)) < Decimal("0.000000001")
assert table.cells[0][2]["error"]["code"] == "FM-E004"
assert table.cells[1][1]["value"] == table.base

# Una tabella a una variabile è uguale alla colonna corrispondente della tabella a due variabili.
one_way = fm.analysis.sensitivity(
    "dcf.fcff",
    base,
    output="/enterprise_value",
    rows=("/discount_rate", ["0.08", "0.09", "0.10"]),
)
assert [row[0] for row in one_way.cells] == [row[1] for row in table.cells]

# La stessa richiesta tramite il punto di accesso generico, come la invia un agente o la CLI.
same = fm.run("analysis.sensitivity", {
    "method": "dcf.fcff",
    "base_input": base,
    "output": "/enterprise_value",
    "rows": {"variable": "/discount_rate", "values": ["0.08", "0.09", "0.10"]},
    "columns": {"variable": "/terminal_growth", "values": ["0.01", "0.02", "0.08"]},
})
assert same.cells == table.cells
```

La cella all'8% e all'8% contiene `FM-E004`, perché un valore terminale di Gordon richiede una
crescita inferiore al tasso di attualizzazione; le altre otto celle contengono valori. I
passaggi del risultato elencano il caso base, poi un passaggio per ogni cella valutata con
successo, con etichetta `Cell (i, j)` (numerazione da 1) ed entrambe le variabili sostituite.

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `method` | Identificativo del metodo da valutare, ad esempio `dcf.fcff`; i metodi di analisi non sono ammessi | obbligatorio |
| `base_input` | Input completo di quel metodo, esattamente come il metodo lo accetta: il caso base | obbligatorio |
| `output` | JSON Pointer RFC 6901 all'output decimale riportato in ogni cella, nel `value` del risultato interno, ad esempio `/enterprise_value` | obbligatorio |
| `rows.variable` | JSON Pointer RFC 6901 all'input decimale che varia lungo le righe, in `base_input`, ad esempio `/discount_rate` | obbligatorio |
| `rows.values` | Valori della variabile di riga, nella sua unità di misura, nell'ordine della tabella; almeno uno | obbligatorio |
| `columns.variable` | Puntatore a un secondo input decimale, diverso dalla variabile di riga | assente: tabella a una variabile |
| `columns.values` | Valori della variabile di colonna, nella sua unità di misura, nell'ordine della tabella; almeno uno | obbligatorio con `columns` |

I valori sono nell'unità di misura della variabile: frazione decimale per un tasso (`"0.08"`
corrisponde all'8%), unità monetarie per un importo. In Python, `rows` e `columns` sono coppie
`(pointer, values)`.

## Output

| Campo | Significato |
| --- | --- |
| `inner_method` | `id` e `version` del metodo valutato, nella versione eseguita da questo motore |
| `output` | Puntatore dell'output riportato in ogni cella |
| `base` | Output per `base_input` senza modifiche, nell'unità di misura dell'output |
| `rows` | Asse delle righe, come indicato |
| `columns` | Asse delle colonne, come indicato; assente per una tabella a una variabile |
| `cells` | `cells[i][j]`: `{"value": ...}` nell'unità di misura dell'output, oppure `{"error": {"code", "message"}}` quando il metodo interno ha rifiutato quella combinazione; una tabella a una variabile ha una sola colonna |
| `failed_cells` | Numero di celle che contengono un errore |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Valore | Default |
| --- | --- | --- |
| Metodo interno | identificativo e versione, valutato tramite il registro | obbligatorio |
| Output | puntatore riportato in ogni cella | obbligatorio |
| Ordine di valutazione | caso base, poi le celle riga per riga, con le colonne nell'ordine indicato | fisso |
| Celle in errore | una cella rifiutata dal metodo interno contiene il codice e il messaggio di errore e la tabella prosegue; `FM-E012` (un difetto di Fairmark) interrompe la richiesta | fisso |
| Codifica delle variabili | scritte nell'input del metodo interno come stringhe decimali | fissa |

L'ultima convenzione riportata è il numero di valutazioni del metodo interno e il numero di
celle in errore. Una tabella richiede righe × colonne + 1 valutazioni (compreso il caso base);
il limite di 10.000 consente al massimo 9.999 celle.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Le variabili di riga e di colonna sono lo stesso puntatore, un asse non ha valori, oppure un campo è mancante o sconosciuto |
| `FM-E007` | `method` non corrisponde ad alcun metodo registrato |
| `FM-E015` | Una variabile di riga o di colonna non si risolve in un decimale in `base_input`, oppure `output` non si risolve in un decimale nel risultato del caso base; il messaggio indica il puntatore |
| `FM-E016` | righe × colonne + 1 supera 10.000 valutazioni oppure valutazioni × valori in `base_input` superano 1.000.000; restituito prima della prima valutazione |
| `FM-E017` | `method` indica un metodo di analisi |

I puntatori delle variabili e la sintassi di `output` sono verificati prima di qualsiasi
valutazione; `output` è risolto sul caso base. Un errore del metodo interno sul caso base è
restituito invariato, con il proprio codice: senza caso base la tabella non ha un riferimento.
Un errore del metodo interno in una cella (ad esempio `FM-E004` in `dcf.fcff`, oppure `FM-E001`
quando un valore è fuori dall'intervallo accettato dal metodo interno) è registrato in quella
cella, salvo `FM-E012`, che interrompe la richiesta.

## Precisione

Ogni cella è precisa quanto il metodo interno (vedi la relativa pagina); la tabella non
aggiunge calcoli propri. Nell'esempio le celle differiscono da un calcolo indipendente a 50
cifre di meno di $10^{-24}$ unità monetarie; i casi di riferimento sono verificati entro
$10^{-9}$ unità monetarie.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/analysis.sensitivity/` (una tabella 5 × 5
  WACC per crescita con due celle rifiutate e una tabella a una variabile), i cui valori attesi
  provengono da `scripts/reference_analysis.py`: `dcf.fcff` ricostruito dalla formula
  documentata con il modulo `decimal` di Python a 50 cifre. Non condivide codice con il motore.
- Un ricalcolo indipendente di ogni cella con numpy-financial (`npf.npv` più il valore
  terminale di Gordon) concorda entro un errore relativo di $10^{-9}$
  (`python/tests/test_cross_check.py`).
- Un test di proprietà richiede che ogni cella sia uguale a un'esecuzione diretta del metodo
  interno sull'input modificato, o al suo codice di errore, e che una tabella a una variabile
  sia uguale alla colonna corrispondente di una tabella a due variabili.
