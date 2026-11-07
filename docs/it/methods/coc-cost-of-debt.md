---
source: methods/coc-cost-of-debt.md
source_hash: 3bd49342e962
---
# Costo del debito (`coc.cost_of_debt`)

Restituisce il costo del debito prima e dopo le imposte. Il tasso prima delle imposte è
fornito direttamente (ad esempio il tasso sui finanziamenti in essere della società) oppure
costruito come tasso privo di rischio più uno spread creditizio. Il tasso dopo le imposte
applica lo scudo fiscale a un'aliquota fornita esplicitamente.

## Formula

$$
k_d = r_f + s \quad\text{(when built from a spread)}
\qquad
k_d^{\text{after tax}} = k_d \,(1 - t)
$$

dove $r_f$ è il tasso privo di rischio, $s$ lo spread creditizio e $t$ l'aliquota fiscale dello
scudo fiscale. La prima espressione si applica quando il tasso è costruito da uno spread. Tutti
i tassi sono tassi annui effettivi, espressi come frazioni decimali.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

# Tasso privo di rischio 3% più uno spread del 2.5%, scudo fiscale al 24%.
r = fm.coc.cost_of_debt("0.24", risk_free_rate="0.03", credit_spread="0.025")
assert r.pre_tax_cost_of_debt == Decimal("0.055")
assert r.after_tax_cost_of_debt == Decimal("0.0418")  # 0.055 × (1 − 0.24)
print(r.conventions[-1])  # tax rate of the tax shield 0.24

# Lo stesso tasso prima delle imposte, fornito direttamente.
direct = fm.coc.cost_of_debt("0.24", pre_tax_cost_of_debt="0.055")
assert direct.after_tax_cost_of_debt == Decimal("0.0418")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `tax_rate` | Aliquota fiscale dello scudo fiscale, frazione decimale da 0 (incluso) a 1 (escluso) | obbligatorio |
| `pre_tax_cost_of_debt` | Costo del debito prima delle imposte, frazione decimale annua; fornire questo campo oppure `risk_free_rate` e `credit_spread` | nessuno |
| `risk_free_rate` | Tasso privo di rischio, frazione decimale annua, insieme a `credit_spread` | nessuno |
| `credit_spread` | Spread creditizio sul tasso privo di rischio, frazione decimale annua, insieme a `risk_free_rate` | nessuno |

L'aliquota fiscale è quella che l'analista considera deducibile ai fini dello scudo fiscale; il
motore non applica alcuna logica fiscale specifica (per le società italiane, l'inclusione
dell'IRAP si decide nell'aliquota fornita). L'aliquota è riportata in `explain.conventions`.

## Output

| Campo | Significato |
| --- | --- |
| `pre_tax_cost_of_debt` | Costo del debito prima delle imposte, frazione decimale annua |
| `after_tax_cost_of_debt` | $k_d (1 - t)$, frazione decimale annua |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Tassi | annui effettivi, frazioni decimali | fissa |
| Costo del debito prima delle imposte | fornito, oppure tasso privo di rischio più spread creditizio | nessuno: uno dei due va fornito |
| Aliquota fiscale dello scudo fiscale | input | nessuno: va fornita |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Non è usata nessuna delle due modalità di indicazione del tasso prima delle imposte, oppure sono usate entrambe; è fornito solo uno tra `risk_free_rate` e `credit_spread`; l'aliquota fiscale non è compresa tra 0 (incluso) e 1 (escluso); oppure un campo è mancante o sconosciuto |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Le formule usano solo addizioni e moltiplicazioni: i risultati sono esatti fino alle 28 cifre
significative del tipo decimale.

## Verificato con

- Caso di riferimento (golden case) in `tests/golden/coc.cost_of_debt/`, i cui valori attesi
  sono calcoli in forma chiusa esplicitati nel file.
