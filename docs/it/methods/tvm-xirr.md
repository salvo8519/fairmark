---
source: methods/tvm-xirr.md
source_hash: cd50e9ed335c
---
# Tasso interno di rendimento di flussi datati (`tvm.xirr`)

Trova il tasso annuo al quale il valore attuale netto di flussi di cassa datati è nullo. È la
controparte per flussi datati di [`tvm.irr`](tvm-irr.md): si usa per investimenti i cui flussi
non cadono su periodi regolari, come un commitment in un fondo con richiami e distribuzioni
irregolari. L'algoritmo è deterministico, rifiuta gli input per i quali più di un tasso
risolve l'equazione e dichiara se l'unicità del tasso restituito è dimostrata. Con i valori di
default il risultato coincide con quello della funzione `XIRR` dei fogli di calcolo.

Versione del metodo: 2. La versione 2 rifiuta un `guess` fuori dal dominio del tasso (guess
per periodo di capitalizzazione pari o inferiore a −100%) con `FM-E003` prima di ogni
iterazione; la versione 1 ignorava un guess di questo tipo. I risultati per ogni altro guess
non cambiano.

## Formula

Trova $r$ tale che

$$
\sum_{j} CF_j \cdot DF(r, t_j) = 0,
\qquad t_j = \text{year fraction}(D_1, D_j)
$$

dove $D_1$ è la data del primo flusso e $DF$ è il fattore di sconto di
[`tvm.xnpv`](tvm-xnpv.md): $(1 + r/m)^{-m t}$, oppure $e^{-r t}$ con capitalizzazione continua.
Con i valori di default (Actual/365 (Fixed), capitalizzazione annuale):

$$
\sum_{j} \frac{CF_j}{(1 + r)^{(D_j - D_1)/365}} = 0
$$

## Esempio

Un'uscita di 10000 il 1° gennaio 2008 e quattro entrate fino al 1° aprile 2009.

```python
from decimal import Decimal

import fairmark as fm

flows = [
    ("2008-01-01", -10000),
    ("2008-03-01", 2750),
    ("2008-10-30", 4250),
    ("2009-02-15", 3250),
    ("2009-04-01", 2750),
]
r = fm.tvm.xirr(flows)
assert abs(r.xirr - Decimal("0.37336253351883151031")) < Decimal("1e-15")
assert abs(r.xirr - Decimal("0.373362535")) < Decimal("0.00000001")  # valore pubblicato
assert r.uniqueness_verified                # un solo cambio di segno in ordine di data
print(r.solver, r.iterations, r.tolerance)  # newton 6 1E-18
print(r.xnpv_at_xirr)                       # circa zero: verifica del risultato
```

Il valore pubblicato è 0.373362535. Excel interrompe le iterazioni quando il tasso è preciso
allo 0.000001 per cento, quindi la nona cifra decimale non è significativa: il valore esatto è
0.37336253351883..., che arrotondato a nove decimali dà 0.373362534.

## Algoritmo

Lo stesso algoritmo deterministico di [`tvm.irr`](tvm-irr.md#algoritmo), con due differenze:

- **La griglia è definita su tassi annui effettivi** (−0.99, −0.95, da −0.90 a 1.00 passo
  0.05, da 1.25 a 10 passo 0.25) e convertita nella capitalizzazione scelta prima della
  scansione. L'intervallo di tassi economici esplorato, e quindi il giudizio sull'unicità, non
  dipende da come è espresso il tasso. Quando è trovata una sola radice, `explain` mostra dove:
  il punto della griglia, oppure i due tassi dell'intervallo della griglia con l'XNPV in
  ciascuno di essi.
- **I cambi di segno sono contati sui flussi in ordine di data**; i flussi con la stessa data
  sono compensati.

La regola di unicità è quella di `tvm.irr`: più di una radice trovata, uno scarto dispari tra i
cambi di segno $s$ e le radici trovate, oppure nessuna radice trovata con $s > 1$ danno
`FM-E011`. Una radice trovata con uno scarto pari e positivo è restituita con
`uniqueness_verified = false`, perché non si possono escludere ulteriori tassi a coppie. Con
$s = 1$ l'unicità del tasso è dimostrata.

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `cash_flows` | Flussi datati `{date, amount}`, in qualsiasi ordine; almeno un importo positivo e uno negativo | obbligatorio |
| `day_count` | `act_365_fixed`, `act_360`, `act_act_isda`, `thirty_360` oppure `thirty_e_360` | `act_365_fixed` |
| `compounding` | Capitalizzazione del tasso restituito: `annual`, `semi_annual`, `four_monthly` (3 all'anno), `quarterly`, `bimonthly` (ogni due mesi, 6 all'anno), `monthly` oppure `continuous` | `annual` |
| `guess` | Punto di partenza del metodo di Newton, frazione decimale annua nella capitalizzazione del risultato, maggiore di $-m$, con $m$ il numero di periodi di capitalizzazione all'anno (−1 annuale, −2 semestrale, −3 quadrimestrale, −4 trimestrale, −6 bimestrale, −12 mensile; nessun limite con la capitalizzazione continua) | `0.10` |

## Output

| Campo | Significato |
| --- | --- |
| `xirr` | Tasso annuo al quale l'XNPV è nullo, frazione decimale, con la capitalizzazione scelta |
| `uniqueness_verified` | `true` quando nessun altro tasso può risolvere l'equazione; `false` quando non si possono escludere ulteriori tassi a coppie |
| `xnpv_at_xirr` | XNPV al tasso restituito (circa zero) |
| `solver` | `newton`, `bisection` oppure `grid (exact root)` |
| `iterations` | Iterazioni dell'algoritmo che ha prodotto il risultato |
| `tolerance` | Criterio di arresto sul passo del tasso, $10^{-18}$ |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Calcolo dei giorni | cinque convenzioni, vedi [`dates.year_fraction`](dates-year-fraction.md) | Actual/365 (Fixed), il default dei fogli di calcolo ECMA-376 |
| Capitalizzazione | annuale, semestrale, quadrimestrale, trimestrale, bimestrale, mensile, continua | annuale, il default dei fogli di calcolo ECMA-376 |
| Origine dei tempi | data del primo flusso | fissa |
| Griglia dell'algoritmo | tassi annui effettivi da −0.99 a 10, convertiti nella capitalizzazione | fissa |
| Criterio di arresto | passo sul tasso inferiore a $10^{-18}$, al massimo 200 iterazioni | fisso |
| Punto di partenza del metodo di Newton | `guess` | `0.10` |

Ogni convenzione è riportata indicando se è stata fornita o se è il default, e il risultato
dichiara se l'unicità è dimostrata o non verificata, con il motivo.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Un campo è mancante o sconosciuto, oppure una convenzione non è tra le opzioni elencate |
| `FM-E002` | `cash_flows` è vuoto |
| `FM-E003` | `guess` è pari o inferiore a $-m$, con $m$ il numero di periodi di capitalizzazione all'anno (con capitalizzazione annua, `guess` ≤ −1): il metodo di Newton non può partire da lì; il messaggio indica `/guess` e suggerisce un guess vicino al tasso atteso oppure di omettere `guess` per usare il default 0.10. Verificato prima di ogni iterazione |
| `FM-E005` | Nessuna radice è stata isolata sulla griglia e il metodo di Newton a partire da `guess` non converge; il tasso può trovarsi fuori dall'intervallo dei decimali o lontano da `guess`: indica un `guess` vicino al tasso atteso |
| `FM-E006` | `cash_flows` non contiene alcun importo positivo o alcun importo negativo, così come è fornito oppure dopo la compensazione dei flussi con la stessa data (il messaggio indica quale dei due casi) |
| `FM-E008` | Overflow aritmetico durante il raffinamento della radice |
| `FM-E009` | Una data è malformata, non esiste o è fuori dall'intervallo 1900-01-01 - 2199-12-31 |
| `FM-E011` | Nessun tasso unico può essere isolato: più tassi annullano l'XNPV, è possibile una radice multipla, oppure è noto che esiste un altro tasso oltre a quello trovato; il messaggio elenca gli intervalli e suggerisce `tvm.npv` al costo del capitale o `tvm.mirr` con tassi espliciti |

## Precisione

Le frazioni d'anno sono rapporti esatti. I fattori di sconto con esponente frazionario usano la
funzione potenza decimale, e la capitalizzazione continua l'esponenziale decimale. Rispetto a
un calcolo indipendente a 60 cifre, questi concordano entro circa $10^{-18}$ in termini
relativi o meglio su input tipici; le potenze intere concordano entro circa $10^{-24}$ in
termini relativi o meglio. La tolleranza di $10^{-18}$ è un criterio di arresto sul passo del
tasso, non un limite all'errore, e vicino a una radice doppia la precisione è minore. Il caso
di riferimento a piena precisione è verificato entro $10^{-19}$.

## Verificato con

- Esempio pubblicato: esempio della funzione `XIRR` di ECMA-376 / Excel, 0.373362535.
- Concorda con la funzione `XIRR` di Excel e LibreOffice e con `pyxirr.xirr`.
- Convenzioni di calcolo dei giorni: definizioni delle 2006 ISDA Definitions, sezione 4.16.
