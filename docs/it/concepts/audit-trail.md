---
source: concepts/audit-trail.md
source_hash: 453bfcaee7e4
---
# La tracciabilità del calcolo (audit trail)

Ogni risultato è una busta in quattro parti:

```json
{
  "method": "dcf.fcff",
  "value":   { "enterprise_value": "1421.48…", "equity_value": "1221.48…", "...": "..." },
  "explain": {
    "formula": "EV = Σ FCFF_t × DF_t + TV × DF_n; ...",
    "conventions": ["cash flows discounted at end of period", "..."],
    "steps": [ { "label": "Discount factor, year 1", "formula": "1 / (1 + 0.10)^1", "value": "0.909…" } ]
  },
  "meta": { "engine_version": "…", "method_version": "1", "input_hash": "b963…", "arithmetic": "decimal…" }
}
```

- **`value`** contiene i risultati, esatti, come stringhe decimali.
- **`explain.steps`** elenca ogni calcolo intermedio con i numeri effettivi sostituiti. Un
  analista può rifare a mano qualsiasi passaggio; un agente li cita invece di ricalcolarli.
- **`explain.conventions`** dichiara ogni scelta che cambia il numero (tempistica dei flussi,
  attualizzazione del valore terminale). Le convenzioni non sono mai implicite.
- **`meta`** rende il risultato riproducibile: lo stesso `input_hash` con le stesse versioni
  produce sempre lo stesso valore.

Le etichette dei passaggi sono in inglese, perché fanno parte dell'output del motore. In
Python, `result.explain_markdown()` produce una nota metodologica per la relazione.
