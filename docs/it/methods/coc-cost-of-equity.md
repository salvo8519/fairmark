---
source: methods/coc-cost-of-equity.md
source_hash: d012d1f4879e
---
# Costo del capitale proprio (`coc.cost_of_equity`)

Restituisce il costo del capitale proprio secondo il capital asset pricing model (CAPM), con i
premi che la prassi aggiunge per le società non quotate: premio per il rischio paese, premio
dimensionale e premio per il rischio specifico. Ogni premio è un input esplicito e ogni premio,
fornito o assunto al valore di default, è riportato nella tracciabilità del calcolo (audit
trail). Il beta levered è fornito direttamente oppure ricalcolato all'interno del metodo a
partire da un beta unlevered, come in [`coc.beta_relever`](coc-beta-relever.md).

## Formula

$$
k_e = r_f + \beta_L \cdot ERP + CRP \cdot \lambda + SP + SRP
$$

dove $r_f$ è il tasso privo di rischio, $\beta_L$ il beta levered (beta del capitale proprio), $ERP$ il
premio per il rischio di mercato, $CRP$ il premio per il rischio paese, $\lambda$ l'esposizione
della società al rischio paese, $SP$ il premio dimensionale e $SRP$ il premio per il rischio
specifico. Tutti i tassi sono tassi annui effettivi, espressi come frazioni decimali.

Quando è fornito `relever`, il beta levered è

$$
\beta_L = \beta_U + (\beta_U - \beta_D)\, k,
\qquad
k = (1 - t)\,\frac{D}{E}\ \text{(Hamada)}
\quad\text{or}\quad
k = \frac{D}{E}\ \text{(Harris-Pringle)}
$$

## Quando usare ciascun premio

- **Premio per il rischio paese** ed **esposizione** $\lambda$: quando il premio per il rischio
  di mercato si riferisce a un mercato maturo e la società genera i propri flussi di cassa in
  un mercato più rischioso. $\lambda$ commisura il premio alla quota di attività esposta (ad
  esempio la quota di ricavi realizzata in quel paese); il default pari a 1 indica
  un'esposizione totale.
- **Premio dimensionale**: quando l'analista ritiene che le società di piccole dimensioni
  ottengano rendimenti superiori a quelli spiegati dal beta; di norma è tratto da uno studio
  pubblicato sull'effetto dimensione.
- **Premio per il rischio specifico**: per i rischi propri della singola società non colti
  altrove, come la dipendenza da pochi clienti o da una figura chiave. Poiché si tratta di una
  valutazione soggettiva, un valore diverso da zero richiede una motivazione scritta, che viene
  riportata in `explain.conventions`.

Ogni premio compare nelle convenzioni, con il valore fornito o con il default 0, così che chi
legge il risultato possa vedere quali premi sono stati applicati e quali esclusi.

## Esempio

Tasso privo di rischio 3.5%, beta 1.1, premio per il rischio di mercato 5.5%, premio per il
rischio paese 2% con esposizione 0.5, premio dimensionale 2% e premio per il rischio specifico
motivato dell'1%; quindi relevering con Hamada di un beta unlevered di 0.8 a D/E 0.5 e
aliquota fiscale del 24%.

```python
from decimal import Decimal

import fairmark as fm

r = fm.coc.cost_of_equity(
    "0.035",
    "0.055",
    levered_beta="1.1",
    country_risk_premium="0.02",
    country_exposure="0.5",
    size_premium="0.02",
    specific_risk_premium="0.01",
    specific_risk_justification="Dependence on two customers for 60% of revenue",
)
# 0.035 + 1.1 × 0.055 + 0.02 × 0.5 + 0.02 + 0.01
assert r.cost_of_equity == Decimal("0.1355")
print(r.conventions[-1])  # specific risk premium justification: Dependence on two ...

relevered = fm.coc.cost_of_equity(
    "0.03",
    "0.05",
    relever={
        "unlevered_beta": "0.8",
        "debt_to_equity": "0.5",
        "tax_rate": "0.24",
        "formula": "hamada",
    },
)
# beta 0.8 × (1 + 0.76 × 0.5) = 1.104; ke 0.03 + 1.104 × 0.05 = 0.0852
assert relevered.levered_beta == Decimal("1.104")
assert relevered.cost_of_equity == Decimal("0.0852")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `risk_free_rate` | Tasso privo di rischio, frazione decimale annua | obbligatorio |
| `equity_risk_premium` | Premio per il rischio di mercato, frazione decimale annua | obbligatorio |
| `levered_beta` | Beta levered, adimensionale; fornire esattamente uno tra `levered_beta` e `relever` | nessuno |
| `relever` | `{unlevered_beta, debt_to_equity, tax_rate, formula, debt_beta}` per il relevering di un beta unlevered; `formula` è `hamada` oppure `harris_pringle`, `debt_beta` vale 0 per default | nessuno |
| `country_risk_premium` | Premio per il rischio paese, frazione decimale annua, non negativo | `0` |
| `country_exposure` | Esposizione al rischio paese ($\lambda$), adimensionale, non negativa | `1` |
| `size_premium` | Premio dimensionale, frazione decimale annua | `0` |
| `specific_risk_premium` | Premio per il rischio specifico della società, frazione decimale annua | `0` |
| `specific_risk_justification` | Motivazione scritta del premio per il rischio specifico; obbligatoria quando il premio è diverso da zero | nessuno |

## Output

| Campo | Significato |
| --- | --- |
| `cost_of_equity` | Costo del capitale proprio, frazione decimale annua |
| `levered_beta` | Beta levered utilizzato, fornito o ricalcolato |

I passaggi mostrano il beta levered ottenuto con il relevering (quando calcolato), la componente di rischio di
mercato $\beta_L \cdot ERP$, la componente di rischio paese $CRP \cdot \lambda$ e la somma.

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Tassi | annui effettivi, frazioni decimali | fissa |
| Beta | fornito, oppure ottenuto con il relevering secondo Hamada o Harris-Pringle | nessuno: uno dei due va fornito |
| Beta del debito (nel ricalcolo) | qualsiasi valore | 0, debito privo di rischio |
| Premio per il rischio paese, esposizione | qualsiasi valore | 0 e 1 |
| Premio dimensionale | qualsiasi valore | 0 |
| Premio per il rischio specifico | qualsiasi valore, con motivazione scritta se diverso da zero | 0 |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Sono forniti entrambi o nessuno tra `levered_beta` e `relever`; un premio per il rischio specifico diverso da zero è privo di motivazione; il premio per il rischio paese o l'esposizione al rischio paese sono negativi; in `relever`, il rapporto D/E è negativo o l'aliquota fiscale non è compresa tra 0 (incluso) e 1 (escluso); un campo è mancante o sconosciuto |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

La formula, ricalcolo del beta incluso, usa solo addizioni e moltiplicazioni: i risultati sono
esatti fino alle 28 cifre significative del tipo decimale.

## Verificato con

- Casi di riferimento (golden case) in `tests/golden/coc.cost_of_equity/`, i cui valori attesi
  sono calcoli in forma chiusa esplicitati in ciascun file e verificabili a mano.
- Il CAPM con premi additivi e le formule di ricalcolo del beta nella forma comunemente
  pubblicata.
