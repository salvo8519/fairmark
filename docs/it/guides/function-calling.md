---
source: guides/function-calling.md
source_hash: 4dcebfd3f7a7
---
# Function calling

Fairmark genera le definizioni dei tool per qualsiasi LLM che supporti il function calling. Sono
costruite dagli JSON Schema del motore, quindi corrispondono sempre all'implementazione.

| Provider | Definizioni | Formato |
| --- | --- | --- |
| Anthropic Messages API | `fm.tools.anthropic()` | `name`, `description`, `input_schema` |
| Chat Completions di OpenAI e compatibili | `fm.tools.openai(portable=False)` | `{"type": "function", "function": {...}}` |
| OpenAI Responses API | `fm.tools.openai_responses()` | `{"type": "function", "name": ..., "parameters": ...}` |
| Google Gemini | `fm.tools.gemini()` | `name`, `description`, `parametersJsonSchema` |
| Per nome del provider | `fm.tools.for_provider("ollama")` | il formato giusto per quel provider |
| Qualsiasi altro | `fm.tools.definitions(portable=False)` | `name`, `description`, `parameters` (JSON Schema) |

Provider supportati ed esempi completi: [Provider LLM](providers.md).

## Eseguire le chiamate

```python
result = fm.tools.dispatch(name, arguments)   # arguments: dict o stringa JSON
```

`dispatch` non solleva eccezioni su input non validi. Restituisce
`{"error": {"code", "message"}}`, da rimandare al modello come risultato del tool perché
corregga i parametri.

## Metodi in anteprima

Ogni metodo ha uno stato, `available` (disponibile) o `preview` (anteprima), riportato da
`fm.methods()` (vedi lo
[stato dei metodi](https://salvo8519.github.io/fairmark/project/releases/#method-status), in
inglese). La definizione di un metodo in anteprima riporta "(preview)" dopo il titolo e la
frase `fm.tools.PREVIEW_NOTE` ("Status: preview. ...") nella descrizione, così il modello lo
sa prima di chiamare il tool e può dirlo all'utente. Le definizioni non contengono chiavi
aggiuntive, quindi ogni provider le accetta.

```python
[m["id"] for m in fm.methods() if m["status"] == "preview"]
```

## Nomi dei tool

Il metodo `dcf.fcff` diventa il tool `fairmark_dcf_fcff`. Usa `fm.tools.method_id(name)` per
la conversione inversa.

## Gli schemi visti dal modello

Gli schemi sono espansi (senza `$ref`) per la compatibilità tra fornitori. I campi decimali
accettano numeri o stringhe; le descrizioni dichiarano le unità in modo esplicito ("decimal
fraction, 0.09 = 9%"), ed è questo che porta i modelli a estrarre i tassi correttamente. La
variante portabile (`portable=True`, usata per Gemini e per i runtime locali di modelli
open-weight) dichiara i decimali come stringhe, trasforma i valori opzionali (`X or null`) in
semplici `X` (un campo opzionale è semplicemente non obbligatorio) e rimuove `pattern` e
`additionalProperties`. Le date sono stringhe ISO 8601 (`"2026-06-30"`) in entrambe le
varianti.
