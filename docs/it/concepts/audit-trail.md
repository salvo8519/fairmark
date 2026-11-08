---
source: concepts/audit-trail.md
source_hash: 6b99c2c5ee73
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
- **`meta.defaults_applied`** elenca i valori predefiniti documentati che il motore ha
  assegnato perché i relativi campi non erano forniti, nell'ordine dell'input: per ciascuno,
  `field` (il suo JSON pointer, per esempio `/timing`), `value` (il valore assegnato, come
  sarebbe scritto nell'input) e `reason` (una riga). Ciascuno compare anche tra le prime voci di
  `explain.conventions`, con il prefisso "default:". La chiave è omessa quando non si applica
  alcun valore predefinito, come per ogni metodo senza valori predefiniti documentati; un campo
  fornito esplicitamente prevale sempre e non è mai elencato. I metodi DCF la usano per il loro
  input minimo (Decision 24 di SPEC-0001, si veda
  [`dcf.fcff`](../methods/dcf-fcff.md#input-minimo)). Un metodo che assegna voci dell'input a
  destinazioni vi elenca anche ogni voce fornita la cui destinazione è un valore predefinito
  documentato: `field` è il pointer della voce, `value` la destinazione, e la convenzione
  riporta "assigned to" invece di "not given, set to" (R8 di SPEC-0005, si veda
  [`statements.reorganise`](../methods/statements-reorganise.md#valori-predefiniti)).

Le etichette dei passaggi sono in inglese, perché fanno parte dell'output del motore. In
Python, `result.explain_markdown()` produce una nota metodologica per la relazione.
