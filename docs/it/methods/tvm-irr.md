---
source: methods/tvm-irr.md
source_hash: 146d97badcc3
---
# Tasso interno di rendimento (`tvm.irr`)

Trova il tasso per periodo al quale il valore attuale netto di una serie di flussi di cassa
periodici è nullo. L'algoritmo è deterministico: esplora una griglia fissa di tassi, rifiuta
gli input per i quali più di un tasso risolve l'equazione e dichiara se l'unicità del tasso
restituito è dimostrata. Per flussi in date di calendario usa [`tvm.xirr`](tvm-xirr.md); per
dichiarare in modo esplicito i tassi di finanziamento e di reinvestimento usa
[`tvm.mirr`](tvm-mirr.md).

Versione del metodo: 3. La versione 3 rifiuta un `guess` pari o inferiore a −100% con `FM-E003`
prima di ogni iterazione; la versione 2 ignorava un guess di questo tipo (il metodo di Newton
falliva subito e l'algoritmo ricorreva alla bisezione, oppure restituiva `FM-E005`). I risultati
per ogni altro guess non cambiano. La versione 2 ha aggiunto la scansione su griglia, la
verifica di unicità, l'output `uniqueness_verified` e l'errore `FM-E011`. Gli input con un solo
cambio di segno e un TIR compreso nella griglia danno lo stesso risultato della versione 1; gli
input il cui unico tasso è fuori dalla griglia mentre i flussi cambiano segno più di una volta
ora restituiscono `FM-E011`, e i risultati a tassi negativi possono differire nelle ultime cifre
(i fattori di sconto sono calcolati dal reciproco della base).

## Formula

Trova $r$ tale che

$$
\sum_{t=0}^{n} \frac{CF_t}{(1+r)^t} = 0
$$

Il primo flusso avviene in $t = 0$, come in [`tvm.npv`](tvm-npv.md).

## Esempio

```python
from decimal import Decimal

import fairmark as fm

r = fm.tvm.irr([-1000, 300, 400, 500])
print(r.irr)                  # 0.0889633946933499353177656797
print(r.uniqueness_verified)  # True: un solo cambio di segno nei flussi
print(r.solver, r.iterations) # newton 5
assert abs(r.irr - Decimal("0.088963394693")) < Decimal("0.000000000001")

# I flussi -100, 230, -132 hanno due TIR, 10% e 20%: il motore rifiuta di sceglierne uno.
try:
    fm.tvm.irr([-100, 230, -132])
except fm.FairmarkError as err:
    assert err.code == "FM-E011"
    print(err.message)  # no single rate can be isolated: the grid finds roots in {0.10}, {0.20}; ...

# Tre cambi di segno, un tasso isolato sulla griglia (50%); un'ulteriore coppia non si può escludere.
# Qui una coppia esiste (11% e 12%), quindi il risultato è segnalato.
flagged = fm.tvm.irr(["-1", "3.73", "-4.5882", "1.8648"])
assert not flagged.uniqueness_verified
print(flagged.conventions[-1])  # uniqueness not verified: the flows change sign 3 times; ...
```

## Algoritmo

1. **Scansione su griglia.** Il VAN è valutato su una griglia fissa di tassi per periodo (il
   periodo dei flussi di cassa): −0.99, −0.95, poi da −0.90 a 1.00 con passo 0.05, poi da 1.25
   a 10 con passo 0.25 (ADR 0008). Un punto della griglia in cui il calcolo va in overflow
   viene saltato. Ogni cambio di segno tra punti consecutivi della griglia, e ogni punto della
   griglia in cui il VAN è esattamente zero, è una radice trovata. Quando è trovata una sola
   radice, `explain` mostra dove: il punto della griglia, oppure i due tassi dell'intervallo
   della griglia con il VAN in ciascuno di essi.
2. **Verifica di unicità.** Le radici trovate sono confrontate con il numero $s$ di cambi di
   segno nei flussi. Per la regola dei segni di Cartesio, il numero di tassi superiori a −100%
   è pari a $s$ meno un numero pari. Quindi:

    | Radici trovate sulla griglia | Cambi di segno $s$ | Risultato |
    | --- | --- | --- |
    | più di una | qualsiasi | `FM-E011`, con gli intervalli |
    | una | $s - 1$ dispari | `FM-E011`: esiste almeno un altro tasso |
    | nessuna | $s > 1$ | `FM-E011`: esistono zero o più tassi |
    | una | $s - 1$ pari e positivo | tasso restituito, `uniqueness_verified = false` |
    | una o nessuna | $s = 1$ | tasso restituito, `uniqueness_verified = true` |

3. **Raffinamento.** Con un solo intervallo, il metodo di Newton parte da `guess` (predefinito
   `0.10`) e il suo risultato è accettato solo all'interno dell'intervallo chiuso [tasso
   inferiore, tasso superiore], estremi inclusi; altrimenti si applica la bisezione
   all'interno dell'intervallo. Una radice che cade esattamente su un
   punto della griglia è restituita così com'è (algoritmo `grid (exact root)`). Senza
   intervalli e con $s = 1$, il risultato del metodo di Newton a partire da `guess` è accettato
   solo se converge; altrimenti `FM-E005`.
4. **Criterio di arresto.** Le iterazioni si fermano quando il passo sul tasso è al massimo
   $10^{-18}$, al massimo 200 iterazioni per fase.

Il limite residuo è lo scarto pari: quando viene trovato un tasso e $s - 1$ è pari e positivo,
possono esistere ulteriori tassi a coppie (due radici nello stesso intervallo della griglia,
oppure fuori dalla griglia). Non possono essere esclusi, e il risultato lo dichiara con
`uniqueness_verified = false` e una convenzione. Quando il tasso è ambiguo, per la decisione
preferisci il VAN al costo del capitale, oppure [`tvm.mirr`](tvm-mirr.md).

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `cash_flows` | Flussi di cassa, il primo in $t = 0$; almeno uno positivo e uno negativo | obbligatorio |
| `guess` | Punto di partenza del metodo di Newton, frazione decimale per periodo, maggiore di −1 (−100%) | `0.10` |

## Output

| Campo | Significato |
| --- | --- |
| `irr` | Tasso interno di rendimento per periodo, frazione decimale |
| `uniqueness_verified` | `true` quando nessun altro tasso può risolvere l'equazione; `false` quando non si possono escludere ulteriori tassi a coppie |
| `npv_at_irr` | VAN al tasso restituito (circa zero) |
| `solver` | `newton`, `bisection` oppure `grid (exact root)` |
| `iterations` | Iterazioni dell'algoritmo che ha prodotto il risultato |
| `tolerance` | Criterio di arresto sul passo del tasso, $10^{-18}$ |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Valore | Default |
| --- | --- | --- |
| Origine dei tempi | `cash_flows[0]` in $t = 0$ | fissa |
| Griglia dell'algoritmo | tassi per periodo −0.99, −0.95, da −0.90 a 1.00 passo 0.05, da 1.25 a 10 passo 0.25 | fissa |
| Ordine degli algoritmi | metodo di Newton all'interno dell'intervallo della radice, bisezione come alternativa | fisso |
| Criterio di arresto | passo sul tasso al massimo $10^{-18}$, al massimo 200 iterazioni | fisso |
| Punto di partenza del metodo di Newton | `guess`, riportato come fornito o come default | `0.10` |
| Unicità | dimostrata, oppure non verificata con il motivo | riportata in ogni risultato |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Un campo è mancante o sconosciuto |
| `FM-E002` | `cash_flows` è vuoto |
| `FM-E003` | `guess` è pari o inferiore a −100%: il metodo di Newton non può partire da lì; il messaggio indica `/guess` e suggerisce un guess vicino al tasso atteso oppure di omettere `guess` per usare il default 0.10. Verificato prima di ogni iterazione |
| `FM-E005` | Nessuna radice è stata isolata sulla griglia e il metodo di Newton a partire da `guess` non converge; il tasso può trovarsi fuori dall'intervallo dei decimali o lontano da `guess`: indica un `guess` vicino al tasso atteso |
| `FM-E006` | `cash_flows` non contiene alcun importo positivo o alcun importo negativo |
| `FM-E008` | Overflow aritmetico durante il raffinamento della radice |
| `FM-E011` | Nessun tasso unico può essere isolato: più tassi annullano il VAN, è possibile una radice multipla, oppure è noto che esiste un altro tasso oltre a quello trovato; il messaggio elenca gli intervalli e suggerisce `tvm.npv` al costo del capitale o `tvm.mirr` con tassi espliciti |

## Precisione

Ogni valutazione del VAN usa solo potenze intere; rispetto a un calcolo indipendente a 60
cifre, queste operazioni concordano entro circa $10^{-24}$ in termini relativi o meglio. La
tolleranza di $10^{-18}$ è un criterio di arresto sul passo del tasso, non un limite
all'errore: per una radice semplice il tasso restituito è preciso all'incirca entro quel passo,
mentre vicino a una radice doppia (due tassi molto vicini tra loro) la precisione è minore. I
casi di riferimento sono verificati entro $10^{-12}$.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/tvm.irr/`, compreso un caso con due tassi
  ricavato dalle radici del polinomio ($10\%$ e $20\%$).
- Concorda con la funzione `IRR` dei fogli di calcolo (Excel, LibreOffice),
  `numpy_financial.irr` e `pyxirr.irr` sugli input con un solo tasso.
