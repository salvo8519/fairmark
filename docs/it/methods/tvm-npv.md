---
source: methods/tvm-npv.md
source_hash: 06d6445a80c5
---
# Valore attuale netto (`tvm.npv`)

$$
VAN = \sum_{t=0}^{n} \frac{CF_t}{(1+r)^t}
$$

Il primo flusso avviene in $t = 0$ e non viene attualizzato. È diverso dalla funzione `VAN()`
dei fogli di calcolo, che inizia ad attualizzare dal primo valore.

```python
import fairmark as fm

r = fm.tvm.npv("0.07", [-5000, 1200, 1500, 1800, 2100])
print(r.npv)   # 503.0695432159...
```

| Input | Significato |
| --- | --- |
| `rate` | Tasso di attualizzazione per periodo, frazione decimale |
| `cash_flows` | Flussi di cassa, il primo in $t = 0$ |

Le potenze intere sono calcolate con moltiplicazioni esatte, quindi il risultato non ha errori
di arrotondamento oltre alla divisione finale a 28 cifre.
