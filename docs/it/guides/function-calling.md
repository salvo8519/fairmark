---
source: guides/function-calling.md
source_hash: 59a4aca7deec
---
# Function calling

Fairmark genera le definizioni dei tool per qualsiasi LLM che supporti il function calling. Sono
costruite dagli JSON Schema del motore, quindi corrispondono sempre all'implementazione.

| Fornitore | Definizioni | Formato |
| --- | --- | --- |
| Anthropic Messages API | `fm.tools.anthropic()` | `name`, `description`, `input_schema` |
| Chat Completions compatibili con OpenAI | `fm.tools.openai()` | `{"type": "function", "function": {...}}` |
| Qualsiasi altro | `fm.tools.definitions()` | `name`, `description`, `parameters` (JSON Schema) |

## Eseguire le chiamate

```python
result = fm.tools.dispatch(name, arguments)   # arguments: dict o stringa JSON
```

`dispatch` non solleva eccezioni su input non validi. Restituisce
`{"error": {"code", "message"}}`, da rimandare al modello come risultato del tool perché
corregga i parametri.

## Nomi dei tool

Il metodo `dcf.fcff` diventa il tool `fairmark_dcf_fcff`. Usa `fm.tools.method_id(name)` per
la conversione inversa.

## Gli schemi visti dal modello

Gli schemi sono espansi (senza `$ref`) per la compatibilità tra fornitori. I campi decimali
accettano numeri o stringhe; le descrizioni dichiarano le unità in modo esplicito ("decimal
fraction, 0.09 = 9%"), ed è questo che porta i modelli a estrarre i tassi correttamente.
