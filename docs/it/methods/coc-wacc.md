---
source: methods/coc-wacc.md
source_hash: 819f22b49486
---
# Costo medio ponderato del capitale (`coc.wacc`)

Restituisce il costo medio ponderato del capitale (WACC) a partire dal costo del capitale
proprio, dal costo del debito prima delle imposte, dall'aliquota fiscale dello scudo fiscale e
dalla struttura finanziaria obiettivo. La struttura è fornita come rapporto tra debito e
capitale proprio D/E oppure come peso del debito D/(D + E). Il risultato è il tasso di
attualizzazione di [`dcf.fcff`](dcf-fcff.md).

## Formula

$$
WACC = \frac{E}{V}\, k_e + \frac{D}{V}\, k_d \,(1 - t),
\qquad
\frac{D}{V} = \frac{D/E}{1 + D/E},
\qquad
\frac{E}{V} = 1 - \frac{D}{V}
$$

dove $V = D + E$, $k_e$ è il costo del capitale proprio (vedi
[`coc.cost_of_equity`](coc-cost-of-equity.md)), $k_d$ il costo del debito prima delle imposte
(vedi [`coc.cost_of_debt`](coc-cost-of-debt.md)) e $t$ l'aliquota fiscale dello scudo fiscale.
Tutti i tassi sono tassi annui effettivi, espressi come frazioni decimali.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

# Peso obiettivo del debito 30%: 0.7 × 0.10 + 0.3 × 0.06 × (1 − 0.25)
r = fm.coc.wacc("0.10", "0.06", "0.25", debt_weight="0.3")
assert r.wacc == Decimal("0.0835")
assert r.equity_weight == Decimal("0.7")

# D/E obiettivo 0.5, quindi pesi 2/3 e 1/3: 2/3 × 0.0852 + 1/3 × 0.055 × (1 − 0.24)
by_ratio = fm.coc.wacc("0.0852", "0.055", "0.24", debt_to_equity="0.5")
assert by_ratio.wacc == Decimal("0.0707333333333333333333333333")
assert by_ratio.debt_weight == Decimal("0.3333333333333333333333333333")
assert by_ratio.after_tax_cost_of_debt == Decimal("0.0418")
```

Usa lo stesso D/E obiettivo qui e nel ricalcolo del beta per il costo del capitale proprio,
così che i due input descrivano la stessa struttura finanziaria.

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `cost_of_equity` | Costo del capitale proprio, frazione decimale annua | obbligatorio |
| `pre_tax_cost_of_debt` | Costo del debito prima delle imposte, frazione decimale annua | obbligatorio |
| `tax_rate` | Aliquota fiscale dello scudo fiscale, frazione decimale da 0 (incluso) a 1 (escluso) | obbligatorio |
| `debt_to_equity` | D/E obiettivo, decimale, non negativo; fornire esattamente uno tra `debt_to_equity` e `debt_weight` | nessuno |
| `debt_weight` | D/(D + E) obiettivo, frazione decimale da 0 (incluso) a 1 (escluso); fornire esattamente uno tra `debt_to_equity` e `debt_weight` | nessuno |

## Output

| Campo | Significato |
| --- | --- |
| `wacc` | Costo medio ponderato del capitale, frazione decimale annua |
| `equity_weight` | E / (D + E) |
| `debt_weight` | D / (D + E) |
| `debt_to_equity` | D/E implicito nei pesi; confrontarlo con il D/E usato per riportare a leva il beta |
| `after_tax_cost_of_debt` | $k_d (1 - t)$, frazione decimale annua |

I passaggi mostrano il peso del debito (derivato dal D/E quando è fornito in quella forma), il
peso del capitale proprio, il costo del debito dopo le imposte e la somma ponderata.

## Coerenza con il beta

Usare la stessa struttura finanziaria nel costo del capitale proprio e nei pesi: riportare a
leva il beta al D/E implicito nei pesi del WACC (restituito come `debt_to_equity`). I pesi
obiettivo presuppongono un rapporto D/V costante, coerente con Harris-Pringle; Hamada presuppone
un ammontare di debito costante. Combinare un beta Hamada con pesi obiettivo è
un'approssimazione diffusa e tollerata: va dichiarata quando è rilevante.

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Tassi | annui effettivi, frazioni decimali | fissa |
| Pesi | struttura finanziaria obiettivo, come D/E o D/(D + E) | nessuno: uno dei due va fornito |
| Aliquota fiscale dello scudo fiscale | input, senza logica fiscale specifica (IRAP inclusa o meno nell'aliquota fornita) | nessuno: va fornita |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Sono forniti entrambi o nessuno tra `debt_to_equity` e `debt_weight`, il rapporto D/E è negativo, il peso del debito non è compreso tra 0 (incluso) e 1 (escluso), il D/E è così elevato che il peso dell'equity si arrotonda a zero, l'aliquota fiscale non è compresa tra 0 (incluso) e 1 (escluso), oppure un campo è mancante o sconosciuto |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

La formula è razionale: i risultati sono esatti fino alle 28 cifre significative del tipo
decimale. I pesi derivati dal D/E che non sono decimali finiti (come 1/3) sono arrotondati alla
ventottesima cifra significativa.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/coc.wacc/`, i cui valori attesi sono
  calcoli in forma chiusa esplicitati in ciascun file.
