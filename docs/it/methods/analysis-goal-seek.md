---
source: methods/analysis-goal-seek.md
source_hash: c48f47d98f73
---
# Ricerca obiettivo (`analysis.goal_seek`)

Trova il valore di un input decimale di qualsiasi metodo registrato al quale uno dei suoi
output decimali è pari a un valore obiettivo. Domande tipiche: il tasso di attualizzazione al
quale l'enterprise value del DCF è pari al prezzo offerto (il rendimento implicito
dell'offerta), oppure il tasso al quale un VAN è nullo. La ricerca si svolge all'interno di un
intervallo indicato dal chiamante, ogni valutazione del metodo interno è riportata e il
risultato contiene l'input del metodo interno alla soluzione, così che chiunque possa
rieseguirlo.

La ricerca obiettivo (goal seek) è un metodo di analisi: valuta un altro metodo tramite il
registro (vedi [Metodi di analisi](../concepts/analysis.md)). Non può valutare un altro metodo
di analisi.

## Formula

Dati il metodo interno $M$, la variabile $x$ (un input decimale di $M$), l'output $y = M(x)$ e
il valore obiettivo $T$, la ricerca obiettivo trova $x^*$ nell'intervallo di ricerca
$[x_{low}, x_{high}]$ tale che

$$
\lvert f(x^*) \rvert \le \text{tolerance},
\qquad
f(x) = y(x) - T
$$

La tolleranza è assoluta e misurata sull'output, nell'unità di misura dell'output (valuta per
un enterprise value, frazione decimale per un tasso).

## Algoritmo

Metodo di Brent nella formulazione classica dei manuali, quella implementata da
`scipy.optimize.brentq`:

1. Si valutano entrambi gli estremi dell'intervallo di ricerca, prima quello inferiore. Se un
   estremo rispetta già la tolleranza, è restituito con 0 iterazioni. Se $f(x_{low})$ e
   $f(x_{high})$ hanno lo stesso segno in senso stretto, il valore obiettivo non è compreso
   nell'intervallo (`FM-E013`). Non è prevista alcuna ricerca automatica di un intervallo più
   ampio.
2. Ogni iterazione calcola un nuovo punto per interpolazione quadratica inversa sugli ultimi tre
   punti, con il metodo delle secanti sugli ultimi due, oppure per bisezione dell'intervallo
   corrente. Un passo interpolato è accettato solo se resta ben all'interno dell'intervallo e
   lo riduce abbastanza rapidamente; altrimenti l'iterazione applica la bisezione. Anche
   un'interpolazione che andrebbe in overflow ricade sulla bisezione.
3. Il passo sulla variabile non è mai inferiore a $10^{-24} \times \max(1, \lvert x \rvert)$,
   così ogni nuovo punto resta distinto alle 28 cifre significative del tipo decimale. Ogni
   nuovo punto è strettamente interno all'intervallo corrente, che è sempre contenuto in
   quello indicato dal chiamante.
4. La ricerca si ferma al primo punto con $\lvert f(x) \rvert \le$ `tolerance`. Se
   `max_iterations` punti successivi ai due estremi non la rispettano, il risultato è
   `FM-E014`.

L'output non deve essere necessariamente monotono, ma deve essere continuo nella variabile
sull'intervallo di ricerca: quando l'output salta oltre il valore obiettivo, l'intervallo si
riduce fino al passo minimo senza rispettare la tolleranza e il risultato è `FM-E014`, che
indica l'intervallo ridotto, mai una falsa soluzione.

Due limiti da tenere presenti:

- **Più soluzioni.** Quando l'intervallo di ricerca contiene più di una soluzione (flussi di
  cassa non convenzionali possono avere più TIR), ne viene restituita una, determinata dal
  percorso delle iterazioni, e il risultato è lo stesso a ogni esecuzione. Per selezionarne
  un'altra, restringi l'intervallo di ricerca.
- **Output poco sensibili.** La tolleranza si applica all'output, non alla variabile. Dove
  l'output varia appena al variare della variabile, un punto che rispetta la tolleranza può
  essere lontano dalla soluzione esatta; in tal caso riduci la tolleranza.

## Esempio

Il DCF di [`dcf.fcff`](dcf-fcff.md) valuta flussi di cassa operativi disponibili di 100, 110 e
120 con un tasso di crescita di lungo periodo del 2%. A quale tasso di attualizzazione
l'enterprise value è pari a un'offerta di 1300?

```python
from decimal import Decimal

import fairmark as fm

base = {
    "stages": [{"periods": 3, "projection": "given", "flows": [100, 110, 120]}],
    "cash_flow_terms": "nominal",
    "discount_rate_terms": "nominal",
    "wacc": "0.10",
    "timing": "end_of_period",
    "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
    "net_financial_position": {"gross_debt": 200, "cash": 0},
}
r = fm.analysis.goal_seek(
    "dcf.fcff",
    base,
    variable="/wacc",
    output="/enterprise_value",
    target="1300",
    bracket=("0.05", "0.5"),
)
print(r.solution)    # 0.1073730997872031000512537941
print(r.achieved)    # 1300.0000000000600441199177741
print(f"{r.residual:f}")  # 0.0000000000600441199177741, entro la tolleranza di default 0.000000001
print(r.iterations)  # 9, dopo i due estremi dell'intervallo
assert abs(r.solution - Decimal("0.107373099787207080")) < Decimal("0.000000000001")

# La stessa richiesta tramite il punto di accesso generico, come la invia un agente o la CLI.
same = fm.run("analysis.goal_seek", {
    "method": "dcf.fcff",
    "base_input": base,
    "variable": "/wacc",
    "output": "/enterprise_value",
    "target": "1300",
    "bracket": ["0.05", "0.5"],
})
assert same.solution == r.solution

# Riesegue il metodo interno alla soluzione per ottenerne l'explain completo.
inner = fm.run(r.inner_method["id"], r.solved_input)
assert inner.value == r.inner_value
print(inner.steps[0].label)  # primo passaggio della tracciabilità del calcolo del DCF
```

Il tasso di attualizzazione implicito nell'offerta è circa 10,74%. I passaggi del risultato
elencano in ordine le 11 valutazioni (i due estremi dell'intervallo, poi passi con il metodo
delle secanti, di bisezione e di interpolazione quadratica inversa) con il tasso,
l'enterprise value e il residuo di ciascuna.

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `method` | Identificativo del metodo da valutare, ad esempio `dcf.fcff`; i metodi di analisi non sono ammessi | obbligatorio |
| `base_input` | Input completo di quel metodo, esattamente come il metodo lo accetta, compresa la variabile con un valore iniziale qualsiasi | obbligatorio |
| `variable` | JSON Pointer RFC 6901 all'input decimale da determinare, in `base_input`, ad esempio `/discount_rate` o `/cash_flows/0` | obbligatorio |
| `output` | JSON Pointer RFC 6901 all'output decimale, nel `value` del risultato interno, ad esempio `/enterprise_value` | obbligatorio |
| `target` | Valore che l'output deve raggiungere, nell'unità di misura dell'output | obbligatorio |
| `bracket` | `[low, high]` con low < high: l'intervallo di ricerca della variabile, nell'unità di misura della variabile | obbligatorio |
| `tolerance` | Tolleranza assoluta sull'output, nell'unità di misura dell'output, maggiore di 0 | `0.000000001` |
| `max_iterations` | Numero massimo di valutazioni dopo i due estremi dell'intervallo, da 1 a 1.000 | `100` |

Scegli la tolleranza nell'unità di misura dell'output. Per un enterprise value in euro, il
default di $10^{-9}$ euro è molto al di sotto di qualsiasi importo significativo; per un output
che è un tasso, corrisponde a $10^{-7}$ punti percentuali. Una tolleranza $\varepsilon$
sull'output determina la variabile all'incirca entro $\varepsilon / \lvert dy/dx \rvert$.

## Output

| Campo | Significato |
| --- | --- |
| `inner_method` | `id` e `version` del metodo valutato, nella versione eseguita da questo motore |
| `solution` | Valore della variabile al quale l'output raggiunge il valore obiettivo |
| `achieved` | Output del metodo interno alla soluzione |
| `residual` | `achieved − target`; il suo valore assoluto non supera la tolleranza |
| `iterations` | Valutazioni dopo i due estremi dell'intervallo (0 quando un estremo raggiunge il valore obiettivo) |
| `tolerance` | Tolleranza applicata, nell'unità di misura dell'output |
| `solved_input` | Input del metodo interno alla soluzione, con la variabile scritta come stringa decimale |
| `inner_value` | `value` del risultato interno alla soluzione |

`solved_input` e `inner_value` rendono la soluzione riproducibile: eseguendo il metodo interno
su `solved_input` si ottengono `inner_value` e l'explain completo del metodo interno, che la
ricerca obiettivo non ripete.

I passaggi riportano una voce per ogni valutazione, con etichetta `Evaluation k (bracket end)`,
`(bisection)`, `(secant)` o `(inverse quadratic interpolation)`, con la variabile, l'output e
il residuo sostituiti, seguita da un passaggio finale `Solution`.

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Valore | Default |
| --- | --- | --- |
| Metodo interno | identificativo e versione, valutato tramite il registro | obbligatorio |
| Stato del metodo interno | se il metodo interno è in anteprima, "Evaluated method <id> is preview"; lo stato è anche in `meta.evaluated_method_status` | riportato |
| Ricerca della radice | metodo di Brent: interpolazione quadratica inversa, metodo delle secanti, bisezione | fisso |
| Intervallo di ricerca | indicato dal chiamante; nessuna ricerca automatica | obbligatorio |
| Entrambi gli estremi rispettano la tolleranza | è restituito l'estremo inferiore | fisso |
| Più soluzioni nell'intervallo di ricerca | ne è restituita una, determinata dal percorso delle iterazioni | fisso |
| Criterio di arresto | $\lvert \text{output} - \text{target} \rvert \le$ `tolerance`, assoluto, nell'unità di misura dell'output | `0.000000001` |
| Numero massimo di iterazioni | `max_iterations` dopo i due estremi dell'intervallo | `100` |
| Passo minimo sulla variabile | $10^{-24} \times \max(1, \lvert x \rvert)$ nel punto corrente | fisso |
| Codifica della variabile | scritta nell'input del metodo interno come stringa decimale | fissa |
| Ordine di valutazione | estremo inferiore, estremo superiore, poi una valutazione per iterazione | fisso |

L'ultima convenzione riportata è il numero di valutazioni del metodo interno.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | low ≥ high, tolleranza ≤ 0, `max_iterations` fuori dall'intervallo da 1 a 1.000, oppure un campo è mancante o sconosciuto |
| `FM-E007` | `method` non corrisponde ad alcun metodo registrato |
| `FM-E013` | L'output meno il valore obiettivo ha lo stesso segno a entrambi gli estremi dell'intervallo e nessuno dei due rispetta la tolleranza; il messaggio riporta entrambi i valori |
| `FM-E014` | La tolleranza non è stata rispettata entro `max_iterations`, oppure l'output salta oltre il valore obiettivo; il messaggio riporta la valutazione più vicina e l'intervallo che contiene ancora il valore obiettivo |
| `FM-E015` | `variable` non si risolve in un decimale in `base_input`, oppure `output` non si risolve in un decimale nel risultato interno; il messaggio indica il puntatore |
| `FM-E016` | La richiesta supererebbe 10.000 valutazioni del metodo interno; la ricerca obiettivo ne richiede al massimo 1.002, quindi questo codice riguarda gli altri metodi di analisi |
| `FM-E017` | `method` indica un metodo di analisi |

`variable` è verificato prima di qualsiasi valutazione, `output` alla prima. Un errore del
metodo interno in qualsiasi valutazione è restituito invariato, con il proprio codice: ad
esempio `FM-E004` quando un tasso di attualizzazione nell'intervallo di ricerca è pari o
inferiore al tasso di crescita di lungo periodo di `dcf.fcff`. La ricerca di una radice su una
funzione definita solo in parte non è definita: scegli un intervallo di ricerca sul quale il
metodo interno sia valido.

## Precisione

Il criterio di arresto limita il residuo sull'output, non l'errore sulla variabile. Ogni
valutazione è precisa quanto il metodo interno (vedi la relativa pagina). Nell'esempio, la
soluzione differisce da un calcolo indipendente a 50 cifre di circa $4 \times 10^{-15}$; i casi
di riferimento sono verificati entro $10^{-12}$ (DCF) e $10^{-15}$ (TIR tramite VAN, con
tolleranza $10^{-18}$).

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/analysis.goal_seek/`, le cui soluzioni
  attese provengono da `scripts/reference_analysis.py`: bisezione con il modulo `decimal` di
  Python a 50 cifre fino a $10^{-30}$ sulla variabile, con `dcf.fcff` e `tvm.npv` ricostruiti
  dalle formule documentate. Non condivide codice con il motore.
- Il tasso al quale il VAN di −1000, 300, 400, 500 è nullo concorda con
  [`tvm.irr`](tvm-irr.md) e con `pyxirr.irr`.
- Lo stesso script contiene il metodo di Brent scritto a partire dalla formulazione classica
  dei manuali (Numerical Recipes, `zbrent`). Un test richiede che il motore ne riproduca numero
  di iterazioni e soluzione, compreso un caso con tre TIR nell'intervallo di ricerca, così che
  qualsiasi modifica al percorso dell'algoritmo venga rilevata.
