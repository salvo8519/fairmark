---
source: concepts/audit-trail.md
source_hash: 56e2b3cfdbcf
---
# La tracciabilità del calcolo (audit trail)

Ogni risultato è una busta in quattro parti:

```json
{
  "method": "dcf.fcff",
  "value":   { "value_of_operations": "1421.48…", "equity_value": "1221.48…", "...": "..." },
  "explain": {
    "formula": "Value of operations = Σ FCFF_k × DF_k + TV_N × DF_TV; ...",
    "conventions": ["timing: end of period (flow k discounted at t = k)", "..."],
    "steps": [ { "label": "Period 1: discount factor", "formula": "1 / (1 + 0.10)", "value": "0.909…" } ]
  },
  "meta": { "engine_version": "…", "method_version": "3", "input_hash": "b963…", "arithmetic": "decimal…", "method_status": "preview" }
}
```

- **`value`** contiene i risultati, esatti, come stringhe decimali.
- **`explain.steps`** elenca ogni calcolo intermedio con i numeri effettivi sostituiti. Un
  analista può rifare a mano qualsiasi passaggio; un agente li cita invece di ricalcolarli.
- **`explain.conventions`** dichiara ogni scelta che cambia il numero (tempistica dei flussi,
  attualizzazione del valore terminale). Le convenzioni non sono mai implicite.
- **`meta`** rende il risultato riproducibile: lo stesso `input_hash` con le stesse versioni
  produce sempre lo stesso valore. `meta.method_status` indica se il metodo era `available`
  (disponibile) o `preview` (anteprima) al momento dell'esecuzione (vedi lo
  [stato dei metodi](https://salvo8519.github.io/fairmark/project/releases/#method-status), in
  inglese); un risultato in anteprima va presentato come tale.

Le etichette dei passaggi sono in inglese, perché fanno parte dell'output del motore. In
Python, `result.explain_markdown()` produce una nota metodologica per la relazione.
