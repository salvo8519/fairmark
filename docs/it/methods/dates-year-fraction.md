---
source: methods/dates-year-fraction.md
source_hash: 079e6d41d24a
---
# Frazione d'anno (`dates.year_fraction`)

Restituisce la frazione d'anno tra due date di calendario secondo una di cinque convenzioni di
calcolo dei giorni di mercato. Ogni metodo di Fairmark basato su date (`tvm.xnpv`, `tvm.xirr`)
misura il tempo in anni con questa funzione, quindi la scelta della convenzione di calcolo dei
giorni è visibile e verificabile. Il risultato riporta anche i giorni di calendario effettivi
tra le due date, e `explain` mostra numeratore e denominatore in giorni.

## Formula

Per una data iniziale $D_1 = (Y_1, M_1, d_1)$ e una data finale $D_2 = (Y_2, M_2, d_2)$, con
$\text{days} = D_2 - D_1$ in giorni di calendario effettivi:

| Convenzione | `day_count` | Frazione d'anno |
| --- | --- | --- |
| Actual/365 (Fixed) | `act_365_fixed` | $\text{days} / 365$ |
| Actual/360 | `act_360` | $\text{days} / 360$ |
| Actual/Actual (ISDA) | `act_act_isda` | $\dfrac{\text{days in non-leap years}}{365} + \dfrac{\text{days in leap years}}{366}$ |
| 30/360 (Bond Basis) | `thirty_360` | $\dfrac{360(Y_2 - Y_1) + 30(M_2 - M_1) + (d_2 - d_1)}{360}$ |
| 30E/360 (Eurobond Basis) | `thirty_e_360` | stessa formula di 30/360 |

Le due varianti 30/360 differiscono solo nel trattamento del giorno 31:

- **30/360 (Bond Basis):** se $d_1 = 31$, si pone $d_1 = 30$; se $d_2 = 31$ e $d_1 \ge 30$
  (dopo la prima rettifica), si pone $d_2 = 30$.
- **30E/360 (Eurobond Basis):** ogni giorno 31 diventa 30, per entrambe le date.

Con Actual/Actual (ISDA), quando le due date cadono in anni diversi il periodo viene diviso a
ogni 1° gennaio: i giorni fino alla fine del primo anno divisi per la durata di quell'anno, più
uno per ogni anno intero intermedio, più i giorni dell'ultimo anno divisi per la durata di
quell'anno. La data iniziale è inclusa e la data finale è esclusa.

## Esempio

Dal 1° novembre 2003 al 1° maggio 2004, con Actual/Actual (ISDA), ci sono 61 giorni nel 2003
(anno di 365 giorni) e 121 giorni nel 2004 (anno di 366 giorni).

```python
from decimal import Decimal

import fairmark as fm

r = fm.dates.year_fraction("2003-11-01", "2004-05-01", "act_act_isda")
print(r.actual_days)        # 182
print(r.steps[-1].formula)  # 61 / 365 + 0 + 121 / 366
assert round(r.year_fraction, 20) == Decimal("0.49772438056740774010")

# Le due varianti 30/360 differiscono solo sul giorno 31.
bond = fm.dates.year_fraction("2024-02-15", "2024-03-31", "thirty_360")
euro = fm.dates.year_fraction("2024-02-15", "2024-03-31", "thirty_e_360")
assert bond.year_fraction == Decimal(46) / 360
assert euro.year_fraction == Decimal("0.125")  # 45 / 360
```

Le date sono stringhe ISO 8601 (`"2024-06-30"`) oppure oggetti `datetime.date`.

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `start` | Data iniziale, ISO 8601 `YYYY-MM-DD` | obbligatorio |
| `end` | Data finale, ISO 8601 `YYYY-MM-DD`, non anteriore a `start` | obbligatorio |
| `day_count` | `act_365_fixed`, `act_360`, `act_act_isda`, `thirty_360` oppure `thirty_e_360` | obbligatorio |

## Output

| Campo | Significato |
| --- | --- |
| `year_fraction` | Frazione d'anno tra le date, in anni |
| `actual_days` | Giorni di calendario effettivi tra le date |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Valore | Default |
| --- | --- | --- |
| Calendario | date del calendario gregoriano prolettico, senza ora del giorno né fuso orario, dal 1900-01-01 al 2199-12-31 | fissa |
| Calcolo dei giorni | una delle cinque convenzioni sopra | nessuno: deve essere scelta |

I default esistono solo dove una funzione dei fogli di calcolo ne definisce uno: Actual/365
(Fixed) per `tvm.xnpv` e `tvm.xirr`. Actual/Actual (ICMA), che richiede un calendario delle
cedole, e le rettifiche per giorni lavorativi sono fuori ambito.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | La data finale precede la data iniziale, un campo è mancante o sconosciuto, oppure `day_count` non è una delle cinque opzioni |
| `FM-E009` | Una data è malformata (non esattamente `YYYY-MM-DD`), non esiste (`2023-02-29`) o è fuori dall'intervallo 1900-01-01 - 2199-12-31 |

## Precisione

Le frazioni d'anno sono rapporti tra interi, arrotondati una sola volta nella divisione
finale. Rispetto a un calcolo indipendente a 60 cifre, concordano entro circa $10^{-24}$ in
termini relativi o meglio. I casi di riferimento sono verificati entro $10^{-19}$.

## Verificato con

- Definizioni: 2006 ISDA Definitions, sezione 4.16.
- Esempio pubblicato: l'esempio svolto ISDA per Actual/Actual (ISDA), $61/365 + 121/366$.
- Le stesse convenzioni dei day counter di QuantLib; concorda con `pyxirr.year_fraction` per
  tutte e cinque le convenzioni.
