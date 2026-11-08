---
source: methods/tv-limited-horizon.md
source_hash: f2d0b494bc70
---
# Valore terminale con orizzonte finito (`tv.limited_horizon`)

Restituisce il valore terminale all'orizzonte N di un flusso che dura un numero dato n di
ulteriori periodi, seguito da un valore residuo in N + n. La prassi italiana usa questa forma
quando il vantaggio competitivo, una concessione o una licenza hanno una durata limitata: al
posto di una rendita perpetua, il flusso è valutato per n periodi espliciti. Il flusso è
costante, oppure cresce a un tasso dato; il valore residuo è zero se non fornito. Il valore
degli n flussi è il flusso moltiplicato per il fattore di rendita di
[`tvm.annuity`](tvm-annuity.md); si riutilizza solo il fattore, quindi non si calcola alcun
valore in N + n.

## Formula

Con il primo flusso $CF_{N+1}$ alla fine del periodo N + 1:

$$
TV_N = CF_{N+1}\,\frac{1 - \left(\dfrac{1 + g}{1 + r}\right)^{n}}{r - g}
+ \frac{RV_{N+n}}{(1 + r)^{n}}
$$

e $CF_{N+1}\,n / (1 + r)$ per il primo termine quando $g = r$. Al crescere di n il primo
termine tende alla rendita perpetua $CF_{N+1} / (r - g)$ quando $g < r$.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

# Una licenza: flusso 100 per altri 10 periodi all'8%, nulla dopo
r = fm.tv.limited_horizon(100, "0.08", 10)
assert round(r.terminal_value, 10) == Decimal("671.0081398941")

# Flusso crescente, con un valore residuo di 500 in N + 5
g = fm.tv.limited_horizon(100, "0.10", 5, growth="0.02", residual_value=500)
assert round(g.residual_value_discounted, 10) == Decimal("310.4606615296")
print(g.conventions[3])  # residual value at N + 5: 500 (given)

# L'orizzonte più lungo, 10.000 periodi, coincide con la rendita perpetua 100 / 0,10
long = fm.tv.limited_horizon(100, "0.10", 10000)
assert long.terminal_value == Decimal("1000")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `flow` | Primo flusso dopo l'orizzonte, alla fine del periodo N + 1, in unità monetarie | obbligatorio |
| `rate` | Tasso di attualizzazione della fase terminale, frazione decimale per periodo, maggiore di −1 | obbligatorio |
| `periods` | Numero di ulteriori periodi n, da 1 a 10.000 | obbligatorio |
| `growth` | Crescita del flusso per periodo, frazione decimale, maggiore di −1 | `0` (flusso costante) |
| `residual_value` | Valore residuo alla fine del periodo N + n, in unità monetarie | `0` |
| `growth_cap` | Limite alla crescita, frazione decimale per periodo | nessuno, riportato come assente |

## Output

| Campo | Significato |
| --- | --- |
| `terminal_value` | Valore terminale in N, non attualizzato |
| `flows_value` | Valore in N dei flussi da N + 1 a N + n |
| `annuity_factor` | Valore in N di un primo flusso pari a 1 |
| `discount_factor` | $(1 + r)^{-n}$ |
| `residual_value_discounted` | Valore residuo riportato in N |
| `growth_cap` | La crescita (0 se non fornita) confrontata con il limite; `null` senza limite |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Profilo del flusso | costante, oppure crescente a `growth` | costante |
| Numero di periodi | input esplicito | nessuno: va fornito |
| Tempistica | fine dei periodi da N + 1 a N + n; valore residuo in N + n | fissa |
| Valore residuo | fornito, oppure 0 | 0, riportato come default |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | La crescita è pari o inferiore a −100%, oppure un campo manca o è sconosciuto |
| `FM-E003` | Il tasso è pari o inferiore a −100%; il messaggio indica `/rate` |
| `FM-E018` | `periods` è fuori dall'intervallo da 1 a 10.000 (lo stesso codice per entrambi gli estremi) |
| `FM-E008` | Overflow aritmetico (input molto lontani da un intervallo realistico) |

## Precisione

Il valore dei flussi usa il fattore di rendita di [`tvm.annuity`](tvm-annuity.md) moltiplicato
per il flusso; il valore residuo usa un fattore di sconto. Rispetto a una somma termine per
termine a 50 cifre, i casi di riferimento coincidono a $10^{-22}$, compreso un flusso di $10^9$
su 600 periodi. È accettato ogni n da 1 a 10.000: non si calcola alcun valore in N + n, quindi
gli orizzonti lunghi restano entro l'intervallo del tipo decimale. Quando
$((1 + g)/(1 + r))^n$ scende sotto la precisione del tipo decimale (circa $10^{-28}$), è
considerato pari a 0, il suo limite, e un passaggio di explain lo dichiara; il valore coincide
allora con la rendita perpetua entro quella precisione.

## Verificato con

- Casi di riferimento in `tests/golden/tv.limited_horizon/`, da un calcolo indipendente a 50
  cifre che somma i flussi attualizzati uno per uno (`scripts/reference_tv.py`), non la forma
  chiusa usata dal motore.
- Test unitari e Python verificano la convergenza alla rendita perpetua per orizzonti lunghi.
