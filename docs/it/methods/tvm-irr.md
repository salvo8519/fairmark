---
source: methods/tvm-irr.md
source_hash: ac6b196ab504
---
# Tasso interno di rendimento (`tvm.irr`)

Trova $r$ tale che

$$
\sum_{t=0}^{n} \frac{CF_t}{(1+r)^t} = 0
$$

```python
import fairmark as fm

r = fm.tvm.irr([-1000, 300, 400, 500])
print(r.irr)          # 0.08896339469...
print(r.solver, r.iterations)
```

## Algoritmo

1. Metodo di Newton a partire da `guess` (predefinito `0.10`), che si ferma quando il tasso
   varia meno di $10^{-18}$.
2. Se Newton fallisce (derivata nulla, divergenza sotto −100%), bisezione su $[-0.99, 10]$.

Il risultato riporta l'algoritmo usato, le iterazioni e la tolleranza; `npv_at_irr` permette
di verificarlo. Le serie senza cambio di segno non hanno TIR (`FM-E006`).

!!! warning "Più cambi di segno"
    Le serie con più cambi di segno possono avere più TIR. Fairmark restituisce la radice
    trovata a partire da `guess`; passa un `guess` diverso per esplorare le altre e, per le
    decisioni, preferisci il VAN.
