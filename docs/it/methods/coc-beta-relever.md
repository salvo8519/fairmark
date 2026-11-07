---
source: methods/coc-beta-relever.md
source_hash: b9e7907e5ab9
---
# Ricalcolo del beta (`coc.beta_relever`)

Converte un beta dalla forma unlevered (beta non indebitato, o beta dell'attivo) alla forma
levered (beta indebitato, o beta del capitale proprio) a una data struttura finanziaria, o
viceversa. Serve a riportare il beta di settore o delle società comparabili al rapporto D/E
obiettivo della società oggetto di valutazione, oppure a depurare dalla leva un beta osservato.

## Formula

$$
\beta_L = \beta_U + (\beta_U - \beta_D)\, k
\qquad
\beta_U = \frac{\beta_L + \beta_D\, k}{1 + k}
$$

$$
k = (1 - t)\,\frac{D}{E}\ \text{(Hamada)}
\qquad
k = \frac{D}{E}\ \text{(Harris-Pringle)}
$$

dove $\beta_L$ è il beta levered, $\beta_U$ il beta unlevered, $\beta_D$ il beta del debito,
$D/E$ il rapporto tra debito e capitale proprio e $t$ l'aliquota fiscale dello scudo fiscale.
Con $\beta_D = 0$ si ottengono le note espressioni $\beta_L = \beta_U [1 + (1 - t) D/E]$ e
$\beta_L = \beta_U (1 + D/E)$.

## Scelta della formula

Hamada assume un debito costante in valore assoluto: lo scudo fiscale ha quindi lo stesso
rischio del debito e riduce l'effetto della leva del fattore $(1 - t)$. Harris-Pringle assume
un debito ribilanciato verso un rapporto obiettivo: lo scudo fiscale ha il rischio delle
attività operative e l'aliquota fiscale non entra nella formula. Un beta del debito positivo
riconosce che il debito di una società indebitata sopporta parte del rischio d'impresa; a
parità di beta unlevered riduce il beta levered, mentre il default pari a 0 tratta il debito
come privo di rischio.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

# Hamada, debito privo di rischio: 0.8 × (1 + (1 − 0.24) × 0.5)
r = fm.coc.beta_relever(
    "0.8", direction="relever", debt_to_equity="0.5", tax_rate="0.24", formula="hamada"
)
assert r.beta == Decimal("1.104")
print(r.conventions[1])  # debt beta 0 (default: risk-free debt)

# Harris-Pringle con beta del debito 0.2: 0.8 + (0.8 − 0.2) × 0.5
hp = fm.coc.beta_relever(
    "0.8",
    direction="relever",
    debt_to_equity="0.5",
    tax_rate="0.24",
    formula="harris_pringle",
    debt_beta="0.2",
)
assert hp.beta == Decimal("1.1")

# Hamada, depurazione dalla leva con beta del debito 0.1: 1.245 / 1.45
u = fm.coc.beta_relever(
    "1.2",
    direction="unlever",
    debt_to_equity="0.6",
    tax_rate="0.25",
    formula="hamada",
    debt_beta="0.1",
)
assert u.beta == Decimal("0.8586206896551724137931034483")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `beta` | Beta da convertire: unlevered per `relever`, levered per `unlever`; adimensionale | obbligatorio |
| `direction` | `relever` oppure `unlever` | obbligatorio |
| `debt_to_equity` | Rapporto tra debito e capitale proprio D/E, decimale (0.5 = debito pari alla metà del capitale proprio), non negativo | obbligatorio |
| `tax_rate` | Aliquota fiscale dello scudo fiscale, frazione decimale da 0 (incluso) a 1 (escluso) | obbligatorio |
| `formula` | `hamada` oppure `harris_pringle` | obbligatorio |
| `debt_beta` | Beta del debito, adimensionale | `0` (debito privo di rischio) |

L'aliquota fiscale è quella che l'analista considera deducibile ai fini dello scudo fiscale; il
motore non applica alcuna logica fiscale specifica (per le società italiane, l'inclusione
dell'IRAP si decide nell'aliquota fornita). Harris-Pringle non la utilizza, ma l'aliquota viene
comunque validata e riportata.

## Output

| Campo | Significato |
| --- | --- |
| `beta` | Il beta convertito: levered per `relever`, unlevered per `unlever` |

I passaggi mostrano il fattore di leva $k$ e il beta convertito.

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Formula | Hamada, Harris-Pringle | nessuno: va scelta |
| Beta del debito | qualsiasi valore | 0, debito privo di rischio |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Il rapporto D/E è negativo, l'aliquota fiscale non è compresa tra 0 (incluso) e 1 (escluso), la formula o la direzione non è tra le opzioni elencate, oppure un campo è mancante o sconosciuto |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Le formule sono razionali: i risultati sono esatti fino alle 28 cifre significative del tipo
decimale. Depurando dalla leva e poi riportando a leva alla stessa struttura si ottiene il beta
originale entro $10^{-26}$ (property test, entrambe le formule, con e senza beta del debito).

## Verificato con

- Esempi pubblicati (Damodaran, slide pubbliche dei corsi alla NYU Stern): Tata Motors, beta
  unlevered 0.8601 riportato a leva con D/E 41.41% e aliquota 32.45% dà 1.1007; un beta mediano di
  1.24 depurato dalla leva con D/E lordo 27.06% e aliquota 40% dà 1.0668. Entrambi sono golden case.
  Le stesse slide riportano la versione con beta del debito, algebricamente identica a quella usata qui.

- Casi di riferimento (golden case) in `tests/golden/coc.beta_relever/`, i cui valori attesi
  sono calcoli in forma chiusa esplicitati in ciascun file.
- Le formule di Hamada e di Harris-Pringle, con e senza beta del debito, nella forma comunemente
  pubblicata.
