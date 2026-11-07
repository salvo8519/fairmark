---
source: methods/tvm-npv.md
source_hash: 3542c9fb163d
---
# Valore attuale netto (`tvm.npv`)

Attualizza in $t = 0$ una serie di flussi di cassa periodici a un unico tasso per periodo. Per
flussi in date di calendario usa [`tvm.xnpv`](tvm-xnpv.md); per un tasso diverso in ogni
periodo o per la convenzione di metà periodo usa [`tvm.present_value`](tvm-present-value.md).

Versione del metodo: 2. La versione 2 calcola i fattori di sconto per tassi negativi dal
reciproco di $1 + r$; solo le ultime cifre dei risultati con tassi negativi differiscono dalla
versione 1.

## Formula

$$
NPV = \sum_{t=0}^{n} \frac{CF_t}{(1+r)^t}
$$

Il primo flusso avviene in $t = 0$ e non viene attualizzato. È diverso dalla funzione `NPV`
(`VAN`) dei fogli di calcolo, che inizia ad attualizzare dal primo valore.

## Esempio

```python
import fairmark as fm

r = fm.tvm.npv("0.07", [-5000, 1200, 1500, 1800, 2100])
print(r.npv)   # 503.0695432159...
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `rate` | Tasso di attualizzazione per periodo, frazione decimale | obbligatorio |
| `cash_flows` | Flussi di cassa, il primo in $t = 0$ | obbligatorio |

## Output

| Campo | Significato |
| --- | --- |
| `npv` | Valore attuale netto in $t = 0$ |
| `present_values` | Valore attuale di ciascun flusso |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Valore | Default |
| --- | --- | --- |
| Origine dei tempi | `cash_flows[0]` in $t = 0$, non attualizzato | fissa |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Un campo è mancante o sconosciuto |
| `FM-E002` | `cash_flows` è vuoto |
| `FM-E003` | Il tasso è pari o inferiore a −100% |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Le potenze intere sono calcolate con moltiplicazioni esatte. Rispetto a un calcolo
indipendente a 60 cifre, i risultati concordano entro circa $10^{-24}$ in termini relativi o
meglio. I tassi negativi sono attualizzati tramite il reciproco di $1 + r$, mantenendo la piena
precisione relativa, e gli orizzonti molto lunghi tendono a zero (underflow) invece di andare
in overflow. Il caso di riferimento è verificato entro $10^{-9}$.

## Verificato con

- Caso di riferimento (golden case) in `tests/golden/tvm.npv/`.
- Concorda con `numpy_financial.npv` e `pyxirr.npv`, che usano la stessa origine dei tempi, e
  con l'espressione dei fogli di calcolo `NPV(r, CF_1, ..., CF_n) + CF_0` (Excel, LibreOffice).
