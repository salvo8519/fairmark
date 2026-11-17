---
source: concepts/audit-trail.md
source_hash: 82c0e4abd803
---
# La tracciabilità del calcolo (audit trail)

Ogni risultato è una busta (envelope) in cinque parti, in questo ordine (ADR 0014 G4; versione
dell'envelope 1):

```json
{
  "method": "dcf.fcff",
  "value":   { "value_of_operations": "1421.48…", "equity_value": "1221.48…", "...": "..." },
  "explain": {
    "formula": "Value of operations = Σ FCFF_k × DF_k + TV_N × DF_TV; ...",
    "conventions": ["timing: end of period (flow k discounted at t = k)", "..."],
    "steps": [ { "label": "Period 1: discount factor", "formula": "1 / (1 + 0.10)", "value": "0.909…" } ]
  },
  "diagnostics": { "not_computed": [], "warnings": [], "checks": [] },
  "meta": { "engine_version": "…", "envelope_version": 1, "method_version": "3", "method_status": "preview",
            "input_hash": "b963…", "arithmetic": "decimal…", "defaults_applied": [] }
}
```

- **`value`** contiene i risultati, esatti, come stringhe decimali. Il suo JSON Schema
  documenta ogni campo con la sua unità (`fairmark schema <metodo> --output`,
  `fm.output_schema(metodo)`); in Python `result.value` è decodificato in base a esso: ogni
  decimale diventa un `Decimal`, mentre etichette, id, versioni, puntatori e messaggi restano
  testo anche quando sembrano numeri.
- **`explain.steps`** elenca ogni calcolo intermedio con i numeri effettivi sostituiti. Un
  analista può rifare a mano qualsiasi passaggio; un agente li cita invece di ricalcolarli.
- **`explain.conventions`** dichiara ogni scelta che cambia il numero (tempistica dei flussi,
  attualizzazione del valore terminale). Le convenzioni non sono mai implicite.
- **`diagnostics`** dice che cosa il risultato non contiene, che cosa il lettore potrebbe non
  notare e che cosa è stato verificato, in tre elenchi sempre presenti, vuoti quando non c'è
  nulla da segnalare:
  - `not_computed`: una voce per ogni output non calcolato, `{pointer, reason_code, reason,
    missing_any_of, permitted}`. `pointer` è l'output dalla radice dell'envelope
    (`/value/...`); `reason_code` è `missing_input` (gli input che lo renderebbero calcolabile
    sono in `missing_any_of`, insiemi alternativi di pointer dell'input), `choice_required`
    (una classificazione da fornire, con i valori ammessi in `permitted`), `not_defined` (un
    termine fornito rende l'indice non definito, per esempio un capitale proprio negativo) o
    `not_applicable` (nessun input potrebbe fornirlo nella forma data). In ordine di pointer.
  - `warnings`: `{code, pointer, message}`, con un codice stabile `FM-Wnnn` documentato nei
    [codici delle avvertenze](../reference/warnings.md); ciascuna è riportata in
    `explain.conventions` come `warning FM-Wnnn: <message>`.
  - `checks`: ogni relazione che il metodo ha verificato, `{id, status, pointer, pointers}`,
    `status` `reconciled` o `not_checked` (una relazione che non quadra è un errore,
    `FM-E026`, mai un controllo).
- **`meta`** rende il risultato riproducibile: lo stesso `input_hash` con le stesse versioni
  produce sempre lo stesso valore. `meta.method_status` indica se il metodo era `available`
  (disponibile) o `preview` (anteprima) al momento dell'esecuzione (vedi lo
  [stato dei metodi](https://salvo8519.github.io/fairmark/project/releases/#method-status), in
  inglese); un risultato in anteprima va presentato come tale.
- **`meta.envelope_version`** è la versione di questo envelope: una modifica di `explain`,
  `diagnostics` o `meta` segue le regole di versionamento rispetto a essa, non rispetto alla
  versione del metodo (snapshot `schemas/envelope/v1.json`).
- **`meta.defaults_applied`** elenca i valori predefiniti documentati che il motore ha
  assegnato perché i relativi campi non erano forniti, nell'ordine dell'input: per ciascuno,
  `pointer` (il suo JSON pointer, per esempio `/timing`), `value` (il valore assegnato, come
  sarebbe scritto nell'input) e `reason` (una riga). Ciascuno compare anche tra le prime voci di
  `explain.conventions`, con il prefisso "default:". L'elenco è vuoto quando non si applica
  alcun valore predefinito, come per ogni metodo senza valori predefiniti documentati; un campo
  fornito esplicitamente prevale sempre e non è mai elencato. I metodi DCF la usano per il loro
  input minimo (Decision 24 di SPEC-0001, si veda
  [`dcf.fcff`](../methods/dcf-fcff.md#input-minimo)); `dcf.fcff` elenca anche una posizione
  finanziaria netta non fornita, sottratta come zero con un'avvertenza (Decision 25): per
  questa voce `value` (`"0"`) è l'importo sottratto, non un valore da scrivere nel campo, che è
  una struttura, e il valore predefinito non entra in `meta.input_hash`. Un metodo che assegna voci dell'input a
  destinazioni vi elenca anche ogni voce fornita la cui destinazione è un valore predefinito
  documentato: `pointer` è il pointer della voce, `value` la destinazione, e la convenzione
  riporta "assigned to" invece di "not given, set to" (R8 di SPEC-0005, si veda
  [`statements.reorganise`](../methods/statements-reorganise.md#valori-predefiniti)).

Le etichette dei passaggi sono in inglese, perché fanno parte dell'output del motore. In
Python, `result.steps`, `result.conventions`, `result.diagnostics` e `result.meta` contengono la
stessa traccia come dati, con `result.not_computed`, `result.warnings`, `result.checks` e
`result.defaults_applied` come elenchi.
