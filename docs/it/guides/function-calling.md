---
source: guides/function-calling.md
source_hash: b5514c691ef0
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

Ogni funzione qui sopra accetta anche due opzioni per nome, entrambe disattivate per
impostazione predefinita: `compact=True` (schemi più piccoli, vedi
[Profilo compatto](#profilo-compatto)) e `areas=[...]` (solo alcuni tool, vedi
[Sottoinsiemi di tool per area](#sottoinsiemi-di-tool-per-area)).

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

## Valori enumerati e loro significato

Molti campi accettano uno tra un insieme fisso di valori, ciascuno con un significato: i codici
civilistici di `statements.reorganise` (`A1`, `B6`, `B7`, ...), `period_0` e `period_1` in
`series.project`, i tipi di ammortamento di `loans.amortisation_schedule`. Il motore documenta
ogni valore separatamente; la definizione del tool trasforma l'elenco in un semplice `enum` e
aggiunge alla descrizione del campo il significato di ogni valore, così nulla va perso:

```text
`french` (constant instalment), `fixed_amortisation_schedule` (constant principal portion)
or `bullet` (principal repaid with the last instalment). Values: `french` (Constant
instalment (French amortisation, "ammortamento alla francese")); `fixed_amortisation_schedule`
(Constant principal portion ("ammortamento all'italiana")); `bullet` (Interest only; the full
principal is repaid with the last instalment).
```

## Esempi

Ogni definizione contiene esempi di input, presi dai casi di riferimento (golden case) del
metodo:

- l'esempio minimo: un input piccolo e tipico;
- dove utile, un esempio massimo: un input che mostra la maggior parte dei campi opzionali.

Nelle definizioni predefinite si trovano nella parola chiave `examples` dello JSON Schema dei
parametri (`[minimo, massimo]`). Nella variante portabile, dove alcuni fornitori rifiutano le
parole chiave che non conoscono, sono scritti nella descrizione ("Minimal input example:
{...}"), con ogni decimale scritto come stringa, come richiede lo schema portabile.
`fm.tools.examples("dcf.fcff")` li restituisce come dati.

Quale caso di riferimento (golden case) fornisce ogni esempio è stabilito in un'unica tabella,
`TABLE` in `scripts/tool_examples.py`; `python scripts/tool_examples.py` copia gli input in
`python/fairmark/tool_examples.json` (generato, mai modificato a mano, parte di `just
generate`). I test verificano che ogni esempio sia valido rispetto allo schema del metodo e che
il motore lo esegua senza errori.

## Profilo compatto

`compact=True` produce definizioni più piccole per i client con un contesto limitato. Rimuove
solo annotazioni, mai una parola chiave di validazione: accetta quindi esattamente gli stessi
input della definizione senza `compact` e conserva il significato di ogni valore enumerato:

- una descrizione di almeno 40 caratteri, o un elenco di valori enumerati, già data prima nello
  stesso tool compare una sola volta: le occorrenze successive dello stesso campo non hanno
  descrizione;
- `"default": null` è rimosso (assente e null hanno lo stesso significato);
- i valori di `format` non standard (`uint32`) sono rimossi; `minimum` resta valido;
- nella variante portabile, l'indicazione sui decimali ripetuta su ogni importo è dichiarata una
  volta nella descrizione del tool, dove è più breve;
- resta solo l'esempio minimo.

Il `pattern` dei decimali resta su ogni importo: un importo con separatori delle migliaia
(`"1,200"`) o con il segno di percentuale (`"5%"`) è rifiutato prima della chiamata, come lo
rifiuterebbe il motore.

Il risparmio è maggiore per i metodi più grandi: circa un sesto per `statements.reorganise` e
un decimo per `dcf.fcff`, un terzo e un quinto nella variante portabile; i metodi piccoli
cambiano poco. Gran parte di uno schema grande è documentazione non ripetuta, e il profilo
compatto la conserva.

## Sottoinsiemi di tool per area

Un client che usa solo alcuni metodi può chiedere i tool di alcune aree. L'area è la parte
dell'identificativo del metodo prima del punto:

```python
fm.tools.areas()                                   # ["analysis", "coc", "dates", "dcf", ...]
fm.tools.definitions(areas=["dcf", "tvm"])         # solo i tool dcf.* e tvm.*
fm.tools.for_provider("ollama", compact=True, areas=["ratios"])
```

Una selezione vuota o un'area sconosciuta solleva `ValueError` con l'elenco delle aree note.
`dispatch` esegue comunque ogni tool: un sottoinsieme riduce ciò che il modello vede, non ciò
che Fairmark può eseguire.
