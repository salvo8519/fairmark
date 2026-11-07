---
source: methods/rates-real-nominal.md
source_hash: 3dbce11e43e6
---
# Tassi reali e nominali (`rates.real_nominal`)

Converte un tasso nominale in un tasso reale, o un tasso reale in un tasso nominale, data
l'inflazione dello stesso periodo. Fairmark usa la relazione di Fisher esatta, non
l'approssimazione additiva $r \approx n - \pi$, che sovrastima il tasso reale di circa
$r \cdot \pi$ e diventa rilevante quando l'inflazione è elevata. Serve a mantenere coerenti
flussi e tassi di attualizzazione: flussi reali con un tasso reale, flussi nominali con un tasso
nominale.

## Formula

La relazione di Fisher:

$$
(1 + n) = (1 + r)(1 + \pi)
$$

$$
r = \frac{1 + n}{1 + \pi} - 1
\qquad
n = (1 + r)(1 + \pi) - 1
$$

dove $n$ è il tasso nominale, $r$ il tasso reale e $\pi$ il tasso di inflazione, tutti frazioni
decimali riferite allo stesso periodo.

## Esempio

Un tasso nominale dell'8% con inflazione del 3% corrisponde a un tasso reale di
$1.08 / 1.03 - 1 = 4.8544\ldots\%$, non del 5%. Un tasso reale del 2% con inflazione del 3%
corrisponde a un tasso nominale esattamente del 5.06%.

```python
from decimal import Decimal

import fairmark as fm

real = fm.rates.real_nominal("0.08", "0.03", "to_real")
assert round(real.rate, 20) == Decimal("0.04854368932038834951")

nominal = fm.rates.real_nominal("0.02", "0.03", "to_nominal")
assert nominal.rate == Decimal("0.0506")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `rate` | Tasso da convertire, frazione decimale per periodo (0.08 = 8%) | obbligatorio |
| `inflation` | Inflazione dello stesso periodo, frazione decimale (0.03 = 3%) | obbligatorio |
| `direction` | `to_real` (il tasso fornito è nominale) oppure `to_nominal` (il tasso fornito è reale) | obbligatorio |

## Output

| Campo | Significato |
| --- | --- |
| `rate` | Il tasso convertito, frazione decimale per periodo |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Valore | Default |
| --- | --- | --- |
| Relazione | relazione di Fisher esatta, non l'approssimazione additiva | fissa |
| Periodo | tasso e inflazione riferiti allo stesso periodo e alla stessa capitalizzazione | richiesto all'input |

Se il tasso e l'inflazione non si riferiscono allo stesso periodo, converti prima uno dei due
con [`rates.convert`](rates-convert.md).

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Un campo è mancante o sconosciuto, oppure `direction` non è `to_real` né `to_nominal` |
| `FM-E003` | Il tasso o l'inflazione è pari o inferiore a −100% |

## Precisione

La relazione usa solo moltiplicazione, divisione e sottrazione. `to_nominal` è esatto;
`to_real` arrotonda una sola volta, nella divisione finale. Rispetto a un calcolo indipendente
a 60 cifre, i risultati concordano entro circa $10^{-24}$ in termini relativi o meglio. I casi
di riferimento sono verificati entro $10^{-19}$.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/rates.real_nominal/`, da un calcolo
  indipendente a 50 cifre (`scripts/reference_time.py`).
