---
source: methods/tvm-perpetuity.md
source_hash: 1a18088f4486
---
# Rendita perpetua (`tvm.perpetuity`)

Restituisce il valore attuale di un pagamento che prosegue a tempo indefinito, costante o
crescente a un tasso costante inferiore al tasso di attualizzazione. La rendita perpetua
crescente è il modello di crescita di Gordon, usato per il valore terminale e per il valore di
flussi di dividendi stabili. I pagamenti iniziano alla fine del primo periodo (posticipata)
oppure immediatamente (anticipata).

## Formula

Il pagamento del periodo $k$ è $P (1 + g)^{k - 1}$. Per $-1 < g < r$:

$$
PV_{\text{ordinary}} = \frac{P}{r - g}
\qquad
PV_{\text{due}} = \frac{P}{r - g}\,(1 + r)
$$

Con $g = 0$ la formula della rendita posticipata è $P / r$. Quando $g \ge r$ la serie non
converge e il metodo restituisce un errore invece di un valore. Una crescita pari o inferiore a
−100% annulla i pagamenti o ne alterna il segno, quindi è rifiutata come in
[`tvm.annuity`](tvm-annuity.md).

## Esempio

Un primo pagamento di 100, crescente al 2%, attualizzato all'8% (Gordon), e una rendita
perpetua anticipata costante di 100 all'8%:

```python
from decimal import Decimal

import fairmark as fm

gordon = fm.tvm.perpetuity(100, "0.08", timing="ordinary", growth="0.02")
assert round(gordon.present_value, 20) == Decimal("1666.66666666666666666667")

due = fm.tvm.perpetuity(100, "0.08", timing="due")
assert due.present_value == Decimal("1350")
print(due.conventions[-1])  # payment grows by 0 per period (default: level payments)

try:
    fm.tvm.perpetuity(100, "0.05", timing="ordinary", growth="0.05")
except fm.FairmarkError as err:
    assert err.code == "FM-E004"  # la crescita deve essere inferiore al tasso
    print(err.message)
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `payment` | Primo pagamento | obbligatorio |
| `rate` | Tasso di attualizzazione per periodo, frazione decimale; deve essere superiore a `growth` | obbligatorio |
| `growth` | Crescita del pagamento per periodo, frazione decimale, superiore a −1 | `0` (rendita perpetua costante) |
| `timing` | `ordinary` (primo pagamento in $t = 1$) oppure `due` (primo pagamento in $t = 0$) | obbligatorio |

## Output

| Campo | Significato |
| --- | --- |
| `present_value` | Valore attuale in $t = 0$ |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Collocazione temporale | posticipata, anticipata | nessuno: deve essere scelta |
| Crescita | superiore a −100% e inferiore al tasso | 0, riportato come default |

Il tasso e la crescita sono per periodo e devono riferirsi allo stesso periodo e alla stessa
capitalizzazione.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | `growth` è pari o inferiore a −100%, un campo è mancante o sconosciuto, oppure `timing` non è `ordinary` né `due` |
| `FM-E003` | `rate` è pari o inferiore a −100% |
| `FM-E004` | `growth` è pari o superiore a `rate`; il messaggio indica entrambi i campi |

## Precisione

La formula usa una divisione e, per la rendita anticipata, una moltiplicazione. Rispetto a un
calcolo indipendente a 60 cifre, concorda entro circa $10^{-24}$ in termini relativi o meglio.
I casi di riferimento sono verificati entro $10^{-19}$.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/tvm.perpetuity/`, da un calcolo
  indipendente a 50 cifre (`scripts/reference_time.py`).
- La formula chiusa è il modello di crescita di Gordon, la stessa formula usata per il valore
  terminale in [`dcf.fcff`](dcf-fcff.md).
