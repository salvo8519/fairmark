---
source: methods/coc-beta-peers.md
source_hash: aa67630c1740
---
# Beta da società comparabili (`coc.beta_peers`)

Stima il beta di una società non quotata a partire da società comparabili quotate (peers):
esegue l'unlevering del beta osservato di ciascuna comparabile in base alla sua struttura
finanziaria e alla sua aliquota fiscale, calcola la media o la mediana dei beta unlevered e
applica il relevering alla statistica scelta in base alla struttura finanziaria obiettivo della società oggetto di valutazione. Le
formule di unlevering e relevering sono quelle di [`coc.beta_relever`](coc-beta-relever.md).

## Formula

$$
\beta_{U,i} = \frac{\beta_{L,i} + \beta_D\, k_i}{1 + k_i},
\qquad
\bar\beta_U = \operatorname{mean}\ \text{or}\ \operatorname{median}(\beta_{U,i}),
\qquad
\beta_L = \bar\beta_U + (\bar\beta_U - \beta_D)\, k^{*}
$$

dove $k_i = (1 - t_i)\,(D/E)_i$ (Hamada) oppure $(D/E)_i$ (Harris-Pringle) per la comparabile
$i$, e $k^{*}$ è lo stesso fattore calcolato al D/E obiettivo e all'aliquota fiscale della
società. Con un numero pari di comparabili la mediana è la media dei due valori centrali.

## Esempio

Unlevering di cinque comparabili con Hamada; relevering della mediana a D/E 0.5 con aliquota
fiscale del 24%.

```python
from decimal import Decimal

import fairmark as fm

peers = [
    {"name": "Peer A", "levered_beta": "1.15", "debt_to_equity": "0.40", "tax_rate": "0.24"},
    {"name": "Peer B", "levered_beta": "0.95", "debt_to_equity": "0.20", "tax_rate": "0.25"},
    {"name": "Peer C", "levered_beta": "1.30", "debt_to_equity": "0.75", "tax_rate": "0.21"},
    {"name": "Peer D", "levered_beta": "1.05", "debt_to_equity": "0.35", "tax_rate": "0.24"},
    {"name": "Peer E", "levered_beta": "0.88", "debt_to_equity": "0.10", "tax_rate": "0.28"},
]
r = fm.coc.beta_peers(
    peers,
    formula="hamada",
    statistic="median",
    target_debt_to_equity="0.5",
    target_tax_rate="0.24",
)
# La mediana è Peer B: 0.95 / (1 + 0.75 × 0.2) = 0.95 / 1.15; relevering × (1 + 0.76 × 0.5)
assert r.median_unlevered_beta == Decimal("0.8260869565217391304347826087")
assert r.mean_unlevered_beta == Decimal("0.8349189472537530893029774557")
assert r.relevered_beta == Decimal("1.14")
print(r.conventions[-1])  # selected statistic: median (mean of the two central values ...)
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `peers` | Società comparabili, almeno una: `{name, levered_beta, debt_to_equity, tax_rate}`; D/E non negativo, aliquota fiscale da 0 (incluso) a 1 (escluso); `name` è usato solo nella tracciabilità del calcolo | obbligatorio |
| `formula` | `hamada` oppure `harris_pringle`, usata sia per l'unlevering sia per il relevering | obbligatorio |
| `statistic` | `mean` (media) oppure `median` (mediana) dei beta unlevered | obbligatorio |
| `target_debt_to_equity` | D/E obiettivo della società oggetto di valutazione, decimale, non negativo | obbligatorio |
| `target_tax_rate` | Aliquota fiscale della società oggetto di valutazione, frazione decimale da 0 (incluso) a 1 (escluso) | obbligatorio |
| `debt_beta` | Beta del debito, adimensionale, applicato alle comparabili e alla società | `0` (debito privo di rischio) |

## Output

| Campo | Significato |
| --- | --- |
| `unlevered_betas` | Beta unlevered di ciascuna comparabile, nell'ordine di input |
| `mean_unlevered_beta` | Media aritmetica dei beta unlevered |
| `median_unlevered_beta` | Mediana dei beta unlevered |
| `selected_unlevered_beta` | La statistica scelta con `statistic` |
| `relevered_beta` | Beta levered ottenuto con il relevering del beta unlevered selezionato alla struttura finanziaria obiettivo |

Entrambe le statistiche sono sempre restituite, così che l'effetto della scelta sia visibile. I
passaggi mostrano ciascuna comparabile per nome, entrambe le statistiche e il beta riportato a
leva.

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Formula | Hamada, Harris-Pringle | nessuno: va scelta |
| Statistica | media, mediana (media dei due valori centrali con un numero pari di comparabili) | nessuno: va scelta |
| Beta del debito | qualsiasi valore | 0, debito privo di rischio |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | `peers` è vuoto, un rapporto D/E è negativo, un'aliquota fiscale non è compresa tra 0 (incluso) e 1 (escluso), un'opzione non è tra quelle elencate, oppure un campo è mancante o sconosciuto |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Le formule sono razionali: i risultati sono esatti fino alle 28 cifre significative del tipo
decimale. Il caso di riferimento è verificato entro $3 \times 10^{-28}$.

## Verificato con

- Caso di riferimento (golden case) in `tests/golden/coc.beta_peers/`: forma chiusa per
  ciascuna comparabile esplicitata nel file, confermata da un calcolo indipendente a 50 cifre
  (`scripts/reference_coc.py`).
- Le formule di Hamada e di Harris-Pringle nella forma comunemente pubblicata.
