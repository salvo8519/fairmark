# Internal rate of return (`tvm.irr`)

Finds $r$ such that

$$
\sum_{t=0}^{n} \frac{CF_t}{(1+r)^t} = 0
$$

```python
import fairmark as fm

r = fm.tvm.irr([-1000, 300, 400, 500])
print(r.irr)          # 0.08896339469...
print(r.solver, r.iterations)
```

## Solver

1. Newton's method from `guess` (default `0.10`), stopping when the rate changes by less
   than $10^{-18}$.
2. If Newton fails (zero derivative, divergence below −100%), bisection on $[-0.99, 10]$.

The solver used, the iterations and the tolerance are returned, and `npv_at_irr` lets you
check the result. Cash flows without a sign change have no IRR (`FM-E006`).

!!! warning "Multiple sign changes"
    Series with several sign changes can have several IRRs. Fairmark returns the root found
    from `guess`; pass a different `guess` to explore others, and prefer NPV for decisions.
