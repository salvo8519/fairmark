---
source: methods/tv-gordon.md
source_hash: 5264f428b5a7
---
# Valore terminale a crescita costante (`tv.gordon`)

Restituisce il valore terminale all'orizzonte N di un flusso che cresce a un tasso costante g
per sempre, attualizzato al tasso della fase terminale r. È il modello di crescita di Gordon
applicato alla fine della previsione esplicita. Il risultato è un valore in N: attualizzarlo
alla data di valutazione è compito del DCF. Il flusso del primo periodo dopo l'orizzonte è
fornito (ad esempio un flusso normalizzato) oppure ottenuto facendo crescere di g l'ultimo
flusso esplicito; la scelta è obbligatoria ed è riportata.

## Formula

Per $-1 < g < r$:

$$
TV_N = \frac{CF_{N+1}}{r - g},
\qquad
CF_{N+1} = CF_N\,(1 + g) \text{ quando è fornito l'ultimo flusso}
$$

Quando $g \ge r$ la serie non converge e il metodo restituisce un errore invece di un valore.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

# Flusso successivo 50, tasso 8%, crescita 4%, confrontata con un limite del 2% (riportato, non rifiutato)
r = fm.tv.gordon("0.08", "0.04", next_flow=50, growth_cap="0.02")
assert r.terminal_value == Decimal("1250")
assert r.flow_basis == "next_flow_given"
assert r.growth_cap["status"] == "exceeded"

# Ultimo flusso 100 fatto crescere del 2,5% fino a 102,5, tasso 9%
last = fm.tv.gordon("0.09", "0.025", last_flow=100)
assert last.next_flow == Decimal("102.5")
print(last.conventions[1])  # flow at N + 1 is the last flow grown by g: CF_N × (1 + g)
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `rate` | Tasso di attualizzazione della fase terminale, frazione decimale per periodo; può differire da quello della fase esplicita | obbligatorio |
| `growth` | Crescita perpetua, frazione decimale per periodo, maggiore di −1 e inferiore a `rate` | obbligatorio |
| `next_flow` | Flusso in N + 1, in unità monetarie; in alternativa a `last_flow` | nessuno |
| `last_flow` | Flusso in N, in unità monetarie, fatto crescere di `growth`; in alternativa a `next_flow` | nessuno |
| `growth_cap` | Limite alla crescita, frazione decimale per periodo (ad esempio il tasso privo di rischio o la crescita nominale dell'economia) | nessuno, riportato come assente |

## Output

| Campo | Significato |
| --- | --- |
| `terminal_value` | Valore terminale in N, non attualizzato |
| `next_flow` | Flusso in N + 1 utilizzato |
| `flow_basis` | `next_flow_given` oppure `last_flow_grown` |
| `growth_cap` | `{growth, cap, status}` con status `respected` (g ≤ limite) oppure `exceeded`; `null` senza limite |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Flusso in N + 1 | fornito, oppure CF_N × (1 + g) | nessuno: va scelto |
| Tasso della fase terminale | fornito, può differire da quello della fase esplicita | nessuno: va fornito |
| Limite alla crescita | valore oppure nessuno | nessuno, riportato come assente |
| Tempistica | valore in N dei flussi da N + 1 in poi, a fine periodo | fissa |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Sono forniti entrambi o nessuno tra `next_flow` e `last_flow`, la crescita è pari o inferiore a −100%, oppure un campo manca o è sconosciuto |
| `FM-E004` | La crescita è pari o superiore al tasso |
| `FM-E008` | Overflow aritmetico (input molto lontani da un intervallo realistico) |

## Precisione

Una moltiplicazione e una divisione: risultato esatto fino alle 28 cifre significative del
tipo decimale. I casi di riferimento sono verificati a $10^{-22}$ rispetto a un calcolo
indipendente a 50 cifre.

## Verificato con

- `tests/golden/tv.gordon/damodaran-fcff-from-roc-table.yaml`: una cella della tabella "The
  Big Assumption" delle slide delle lezioni di valutazione di Damodaran (NYU Stern), riscritta
  come flusso di Gordon, coincide alla precisione pubblicata.
- Gli altri casi di riferimento provengono da un calcolo indipendente a 50 cifre
  (`scripts/reference_tv.py`). La formula è quella di [`tvm.perpetuity`](tvm-perpetuity.md)
  per una rendita perpetua crescente posticipata.
