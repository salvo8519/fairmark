---
source: methods/tvm-future-value.md
source_hash: fa7cef7d2e87
---
# Valore futuro (`tvm.future_value`)

Restituisce il valore alla fine di $n$ periodi di un importo attuale e di una serie costante di
pagamenti periodici, a un tasso per periodo, con i pagamenti alla fine di ciascun periodo
(rendita posticipata) o all'inizio (rendita anticipata). Si usa per piani di accumulo, fondi di
ammortamento o il saldo di un deposito con prelievi regolari.

## Formula

$$
FV = PV\,(1 + r)^{n} + PMT\,\frac{(1 + r)^{n} - 1}{r}\,\times
\begin{cases}
1 & \text{posticipata} \\
(1 + r) & \text{anticipata}
\end{cases}
\qquad
FV = PV + n\,PMT \text{ quando } r = 0
$$

Il motore capitalizza fino a $t = n$ il fattore di rendita di [`tvm.annuity`](tvm-annuity.md):
i due metodi condividono un'unica implementazione.

**Convenzione sui segni.** Gli importi dello stesso segno si accumulano nella stessa direzione;
un importo negativo è un prelievo. È diversa dalla convenzione sui segni dei flussi dei fogli
di calcolo, dove un versamento è negativo e il valore futuro risulta positivo.

Il tasso è per periodo, come in [`tvm.annuity`](tvm-annuity.md). Per un tasso annuo, convertilo
prima con [`rates.convert`](rates-convert.md) e dividilo per il numero di periodi all'anno.

## Esempio

Gli esempi svolti di ECMA-376, con i segni di questa convenzione:

```python
from decimal import Decimal

import fairmark as fm

savings = fm.tvm.future_value("0.01", 12, payment=1000, timing="ordinary")
assert round(savings.future_value, 2) == Decimal("12682.50")

plan = fm.tvm.future_value("0.005", 10, present_value=500, payment=200, timing="due")
assert round(plan.future_value, 2) == Decimal("2581.40")

# 10.000 versati, 24 prelievi di 500 allo 0,4% per periodo: il deposito va in negativo.
overdrawn = fm.tvm.future_value("0.004", 24, present_value=10000, payment=-500, timing="ordinary")
assert overdrawn.future_value < 0
print(overdrawn.conventions[1])  # sign convention: amounts of the same sign accumulate ...
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `present_value` | Importo a $t = 0$, in unità di valuta | `0` (almeno uno dei due importi) |
| `payment` | Pagamento costante di ogni periodo, in unità di valuta | `0` (almeno uno dei due importi) |
| `rate` | Tasso per periodo, frazione decimale, maggiore di −1 | obbligatorio |
| `periods` | Numero di periodi, da 1 a 10000 | obbligatorio |
| `timing` | `ordinary` (fine di ciascun periodo) o `due` (inizio di ciascun periodo) | obbligatorio |

## Output

| Campo | Significato |
| --- | --- |
| `future_value` | Valore a $t = n$ dell'importo attuale e dei pagamenti |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Momento di pagamento | posticipato, anticipato | nessuno: va scelto |
| Segno | lo stesso segno si accumula, un importo negativo è un prelievo | fissa |
| Importo attuale, pagamento | qualsiasi importo | 0 ciascuno, riportato come default; non entrambi omessi |
| Tasso zero | limite $PV + n\,PMT$, mostrato come passaggio separato | applicato quando $r = 0$ |
| Data del valore futuro | $t = n$ per entrambi i momenti di pagamento | fissa |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | `present_value` e `payment` entrambi omessi, `periods` pari a 0 o superiore a 10000, oppure un campo mancante o sconosciuto |
| `FM-E003` | Il tasso è pari o inferiore a −100% |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Solo potenze intere, moltiplicazioni e divisioni; il fattore di rendita divide per $1 + r$,
quindi i valori che sono decimali finiti risultano esatti a circa $10^{-24}$ in termini
relativi. I casi di riferimento sono verificati a $10^{-20}$ rispetto a un calcolo indipendente
a 50 cifre.

## Verificato con

- Casi di riferimento in `tests/golden/tvm.future_value/`: gli esempi svolti di ECMA-376 della
  funzione del valore futuro e un calcolo indipendente a 50 cifre
  (`scripts/reference_loans.py`).
- `numpy_financial.fv` su input casuali, per entrambi i momenti di pagamento, con i segni
  invertiti.
