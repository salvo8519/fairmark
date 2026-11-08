---
source: methods/tv-liquidation.md
source_hash: 302e97693aff
---
# Valore terminale di liquidazione (`tv.liquidation`)

Restituisce il valore netto di liquidazione all'orizzonte N: i valori di realizzo delle
attività, al netto delle passività estinte, dei costi di liquidazione e delle imposte sulle
plusvalenze realizzate. Si usa per iniziative senza continuità dopo N o con attività
separabili. Ogni voce compare in `explain` con la sua etichetta, così il valore è verificabile
voce per voce.

## Formula

$$
TV_N = \sum \text{valori di realizzo} - \sum \text{passività}
- \sum \text{costi di liquidazione} - \text{imposte sulle plusvalenze}
$$

Il risultato è negativo quando passività e costi superano i valori di realizzo; è restituito
così com'è, senza minimo a zero, e la convenzione lo dichiara.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

r = fm.tv.liquidation(
    assets={"Land": 800, "Plant": 300, "Receivables": 200},
    liabilities={"Bank loan": 600, "Suppliers": 250},
    liquidation_costs={"Legal fees": 40, "Closure": 60},
    tax_on_gains=30,
)
assert r.terminal_value == Decimal("320")
assert r.total_liabilities == Decimal("850")
for step in r.steps[:3]:
    print(step.label, step.value)  # Assets: Land 800, Assets: Plant 300, Assets: Receivables 200

# Sono accettati anche elenchi di coppie (etichetta, importo)
pairs = fm.tv.liquidation([("Lot", 500), ("Lot", 300)], [("Suppliers", 100)], [], 0)
assert pairs.terminal_value == Decimal("700")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `assets` | Elenco di `{label, value}`: valore di realizzo in N di ciascuna attività, zero o positivo | obbligatorio |
| `liabilities` | Elenco di `{label, value}`: importo estinto in N, zero o positivo | obbligatorio |
| `liquidation_costs` | Elenco di `{label, value}`: spese legali, chiusura, indennità al personale, zero o positivi | obbligatorio |
| `tax_on_gains` | Imposte sulle plusvalenze realizzate, come importo, zero o positivo | obbligatorio |

`assets` richiede almeno una voce; gli altri elenchi possono essere vuoti. Le etichette non
possono essere vuote.

## Output

| Campo | Significato |
| --- | --- |
| `terminal_value` | Valore netto di liquidazione in N, non attualizzato |
| `total_assets` | Somma dei valori di realizzo |
| `total_liabilities` | Somma delle passività |
| `total_liquidation_costs` | Somma dei costi di liquidazione |
| `tax_on_gains` | Imposte sulle plusvalenze, come fornite |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Tempistica | attività realizzate e passività estinte in N | fissa |
| Imposte sulle plusvalenze | fornite come importo | fissa: non calcolate da un'aliquota |
| Valore netto negativo | restituito così com'è | fissa: nessun minimo a zero |
| Approccio | lo determinano le passività elencate: includendo il debito finanziario il valore è un equity value | scelta dell'analista |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | `assets` è vuoto, un'etichetta è vuota, oppure un campo manca o è sconosciuto |
| `FM-E018` | Un valore di realizzo, una passività, un costo di liquidazione o le imposte sulle plusvalenze sono negativi |
| `FM-E008` | Overflow aritmetico (input molto lontani da un intervallo realistico) |

## Precisione

Somme e sottrazioni degli importi forniti: risultato esatto.

## Verificato con

- Casi di riferimento in `tests/golden/tv.liquidation/`, con la somma riportata in ciascun file
  e ricalcolata da uno script indipendente (`scripts/reference_tv.py`).
