---
source: methods/tvm-annuity.md
source_hash: 8dda73685601
---
# Rendita (`tvm.annuity`)

Restituisce il valore attuale, il valore futuro e il fattore di rendita di una serie di $n$
pagamenti periodici, costanti o crescenti a un tasso costante, corrisposti alla fine di ciascun
periodo (rendita posticipata) o all'inizio (rendita anticipata). Si usa per canoni di leasing,
piani di rateizzazione, royalty costanti o qualsiasi serie di pagamenti con un numero fisso di
periodi.

## Formula

Il pagamento del periodo $k$ è $P (1 + g)^{k - 1}$. Per una rendita posticipata (pagamenti in
$t = 1, \ldots, n$):

$$
PV_{\text{ordinary}} =
\begin{cases}
P \, \dfrac{1 - \left(\dfrac{1 + g}{1 + r}\right)^{n}}{r - g} & r \ne g \\[2ex]
P \, \dfrac{n}{1 + r} & r = g
\end{cases}
$$

Una rendita anticipata (pagamenti in $t = 0, \ldots, n - 1$) vale un periodo in più:

$$
PV_{\text{due}} = PV_{\text{ordinary}} \,(1 + r)
\qquad
FV = PV \,(1 + r)^{n}
$$

Il fattore è il valore attuale di un primo pagamento pari a 1. Con $g = 0$ la formula della
rendita posticipata è la consueta $P \left[1 - (1 + r)^{-n}\right] / r$.

Quando $r$ e $g$ differiscono per meno di $10^{-9}$, la formula chiusa perde cifre per
cancellazione sia al numeratore sia al denominatore. Il fattore è allora calcolato sommando
direttamente la serie:

$$
PV_{\text{ordinary}} = \frac{P}{1 + r} \sum_{k=1}^{n} \left(\frac{1 + g}{1 + r}\right)^{k - 1}
$$

## Esempio

Un pagamento di 1000 per 10 periodi al 5%, posticipato e anticipato, e una rendita crescente di
20 pagamenti a partire da 1000, con crescita del 3%, attualizzata all'8%:

```python
from decimal import Decimal

import fairmark as fm

ordinary = fm.tvm.annuity(1000, "0.05", 10, timing="ordinary")
due = fm.tvm.annuity(1000, "0.05", 10, timing="due")
growing = fm.tvm.annuity(1000, "0.08", 20, timing="ordinary", growth="0.03")

assert abs(ordinary.present_value - Decimal("7721.73492918481251282906")) < Decimal("1e-15")
assert abs(due.present_value - Decimal("8107.82167564405313847052")) < Decimal("1e-15")
assert abs(growing.present_value - Decimal("12250.04144458699209117333")) < Decimal("1e-15")
print(ordinary.conventions[-1])  # payment grows by 0 per period (default: level payments)

# Caso limite tasso = crescita: n P / (1 + r).
limit = fm.tvm.annuity(1000, "0.06", 15, timing="ordinary", growth="0.06")
assert round(limit.present_value, 20) == Decimal("14150.94339622641509433962")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `payment` | Primo pagamento | obbligatorio |
| `rate` | Tasso di attualizzazione per periodo, frazione decimale | obbligatorio |
| `periods` | Numero di pagamenti, da 1 a 10000 | obbligatorio |
| `growth` | Crescita del pagamento per periodo, frazione decimale, superiore a −1 | `0` (rendita costante) |
| `timing` | `ordinary` (fine di ciascun periodo, rendita posticipata) oppure `due` (inizio di ciascun periodo, rendita anticipata) | obbligatorio |

## Output

| Campo | Significato |
| --- | --- |
| `present_value` | Valore attuale in $t = 0$ |
| `future_value` | Valore in $t = n$ degli stessi pagamenti: valore attuale $\times (1 + r)^n$ |
| `factor` | Valore attuale di un primo pagamento pari a 1 |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Collocazione temporale | posticipata, anticipata | nessuno: deve essere scelta |
| Crescita | qualsiasi tasso superiore a −100% | 0, riportato come default |
| Tasso uguale alla crescita | limite $n / (1 + r)$, mostrato come passaggio separato | applicato quando $r = g$ |
| Tasso prossimo alla crescita | somma diretta, mostrata come passaggio separato | applicata quando $\lvert r - g \rvert < 10^{-9}$ |
| Data del valore futuro | $t = n$ per entrambe le collocazioni | fissa |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | `periods` è 0 o superiore a 10000, `growth` è pari o inferiore a −100%, oppure un campo è mancante o sconosciuto |
| `FM-E003` | Il tasso è pari o inferiore a −100% |
| `FM-E008` | Overflow aritmetico: input molto al di fuori di un intervallo realistico, oppure un montante oltre l'intervallo dei decimali su un orizzonte lungo (vedi [Orizzonti lunghi](#orizzonti-lunghi)) |

## Precisione

La formula chiusa e la somma diretta usano solo potenze intere, moltiplicazioni e divisioni.
Rispetto a un calcolo indipendente a 60 cifre, concordano entro circa $10^{-24}$ in termini
relativi o meglio. I casi di riferimento sono verificati entro $10^{-19}$.

Il fattore di rendita (il valore attuale di un flusso pari a 1 per periodo, in crescita al
tasso $g$) è condiviso con [`tv.limited_horizon`](tv-limited-horizon.md), che lo moltiplica
per il flusso senza calcolare il montante: il suo intervallo di periodi non è quindi limitato
dal montante in $t = n$.

### Orizzonti lunghi

Ogni chiamata calcola il montante $PV \,(1 + r)^n$, e il risultato deve rientrare
nell'intervallo dei decimali (circa $7{,}9 \times 10^{28}$). Su orizzonti lunghi non vi rientra:
una rata di 100 al 5% per periodo, per esempio, restituisce `FM-E008` oltre 1.208 periodi con
rendita posticipata e oltre 1.207 periodi con rendita anticipata. Ne risente anche il valore
attuale: la chiamata non restituisce alcun risultato, quindi neppure il valore attuale, che è
finito. Per il valore attuale di una rendita posticipata lunga, usa
[`tv.limited_horizon`](tv-limited-horizon.md), che calcola lo stesso fattore e mai il montante
(per la rendita anticipata, moltiplica il risultato per $1 + r$). Separare i due valori
cambierebbe gli output del metodo e richiede una nuova versione del metodo; è registrato come
attività successiva nel piano della SPEC-0002.

Quando il montante rientra ancora nell'intervallo ma $((1 + g)/(1 + r))^n$ scende sotto la
precisione dei decimali (circa $10^{-28}$), la potenza è considerata pari a 0, il suo limite, e
un passaggio di explain lo dichiara, come in `tv.limited_horizon`.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/tvm.annuity/`, da un calcolo
  indipendente a 50 cifre (`scripts/reference_time.py`).
- Le rendite costanti concordano con le funzioni `PV` e `FV` dei fogli di calcolo (Excel,
  LibreOffice) e con `numpy_financial.pv` e `numpy_financial.fv`, posticipate e anticipate.
