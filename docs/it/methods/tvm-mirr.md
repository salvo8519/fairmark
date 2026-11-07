---
source: methods/tvm-mirr.md
source_hash: a1de6a18dbc3
---
# Tasso interno di rendimento modificato (`tvm.mirr`)

Restituisce il tasso interno di rendimento modificato (TIRM, MIRR) di una serie di flussi di
cassa periodici: il tasso che porta il valore attuale delle uscite, finanziate a un tasso di
finanziamento, al valore futuro delle entrate, reinvestite a un tasso di reinvestimento. A
differenza del TIR, dichiara in modo esplicito i tassi ai quali il denaro viene preso a
prestito e reinvestito, e ha sempre un unico valore. La definizione è quella della funzione
`MIRR` dei fogli di calcolo.

## Formula

Per i flussi $CF_0, \ldots, CF_n$, il primo in $t = 0$:

$$
PV^{-} = \sum_{CF_t < 0} \frac{CF_t}{(1 + r_{\text{fin}})^{t}}
\qquad
FV^{+} = \sum_{CF_t > 0} CF_t \,(1 + r_{\text{re}})^{\,n - t}
$$

$$
MIRR = \left(\frac{FV^{+}}{-PV^{-}}\right)^{1/n} - 1
$$

dove $r_{\text{fin}}$ è il tasso di finanziamento e $r_{\text{re}}$ il tasso di reinvestimento,
entrambi per periodo.

## Esempio

Un investimento di 120000 seguito da cinque anni di proventi, finanziato al 10% e reinvestito
al 12%. I valori pubblicati sono 12.6094%; considerando solo i primi tre anni, −4.8045%; con
reinvestimento al 14%, 13.4759%.

```python
from decimal import Decimal

import fairmark as fm

flows = [-120000, 39000, 30000, 21000, 37000, 46000]

r = fm.tvm.mirr(flows, finance_rate="0.10", reinvest_rate="0.12")
assert round(r.mirr, 6) == Decimal("0.126094")
assert abs(r.mirr - Decimal("0.12609413036590514583")) < Decimal("1e-15")
print(r.pv_negative_flows)  # -120000
print(r.fv_positive_flows)  # 217297.49504000

three_years = fm.tvm.mirr(flows[:4], finance_rate="0.10", reinvest_rate="0.12")
assert round(three_years.mirr, 6) == Decimal("-0.048045")

reinvest_14 = fm.tvm.mirr(flows, finance_rate="0.10", reinvest_rate="0.14")
assert round(reinvest_14.mirr, 6) == Decimal("0.134759")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `cash_flows` | Flussi di cassa periodici, il primo in $t = 0$; almeno due, con almeno un valore positivo e uno negativo | obbligatorio |
| `finance_rate` | Tasso pagato sul capitale investito (applicato ai flussi negativi), frazione decimale per periodo | obbligatorio |
| `reinvest_rate` | Tasso ottenuto sul reinvestimento dei flussi positivi, frazione decimale per periodo | obbligatorio |

## Output

| Campo | Significato |
| --- | --- |
| `mirr` | Tasso interno di rendimento modificato per periodo, frazione decimale |
| `pv_negative_flows` | Valore attuale in $t = 0$ dei flussi negativi, al tasso di finanziamento |
| `fv_positive_flows` | Valore in $t = n$ dei flussi positivi, al tasso di reinvestimento |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Valore | Default |
| --- | --- | --- |
| Origine dei tempi | `cash_flows[0]` in $t = 0$, come in [`tvm.npv`](tvm-npv.md) | fissa |
| Flussi negativi | attualizzati in $t = 0$ al tasso di finanziamento | fisso |
| Flussi positivi | capitalizzati in $t = n$ al tasso di reinvestimento | fisso |
| Tassi di finanziamento e di reinvestimento | input obbligatori | nessuno |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Meno di due flussi di cassa, oppure un campo è mancante o sconosciuto |
| `FM-E003` | Il tasso di finanziamento o il tasso di reinvestimento è pari o inferiore a −100%; entrambi sono verificati prima di qualsiasi calcolo |
| `FM-E006` | I flussi non contengono alcun valore positivo o alcun valore negativo |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Il valore attuale e il valore futuro usano potenze intere esatte; rispetto a un calcolo
indipendente a 60 cifre, concordano entro circa $10^{-24}$ in termini relativi o meglio. La
radice $n$-esima è calcolata come $\exp(\ln(x)/n)$ con il logaritmo e l'esponenziale decimali,
che concordano entro circa $10^{-18}$ in termini relativi o meglio su input tipici. I casi di
riferimento sono verificati entro $10^{-19}$.

## Verificato con

- Esempi pubblicati: esempi della funzione `MIRR` di ECMA-376 / Excel, 12.6094%, −4.8045% e
  13.4759%.
- Concorda con la funzione `MIRR` di Excel e LibreOffice, `numpy_financial.mirr` e
  `pyxirr.mirr`.
