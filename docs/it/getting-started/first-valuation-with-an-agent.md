---
source: getting-started/first-valuation-with-an-agent.md
source_hash: d3694fcad692
---
# La tua prima valutazione con un agente

In questo tutorial un LLM legge una breve sintesi di business plan, estrae gli input, chiama
Fairmark e scrive una nota di valutazione. Il modello non calcola mai un numero da solo.

## 1. Installa

```bash
pip install fairmark anthropic
export ANTHROPIC_API_KEY=...
```

## 2. Dai al modello i tool di Fairmark

`fm.tools.anthropic()` restituisce le definizioni dei tool generate dagli schemi del motore.
La descrizione di ogni tool chiede al modello di usare il tool invece di fare i calcoli.

```python
import json

import anthropic
import fairmark as fm

client = anthropic.Anthropic()
messages = [{
    "role": "user",
    "content": (
        "Il nostro business plan prevede flussi di cassa operativi di 1,2, 1,35, 1,5, 1,6 e "
        "1,7 milioni di euro. Il WACC è 9,5%, la crescita di lungo periodo 2%, i flussi "
        "arrivano in modo uniforme durante l'anno, la posizione finanziaria netta è 3,1 "
        "milioni. Valuta il capitale proprio e spiega il calcolo."
    ),
}]
```

## 3. Esegui il ciclo

```python
while True:
    response = client.messages.create(
        model="claude-opus-5-5", max_tokens=4096,
        tools=fm.tools.anthropic(), messages=messages,
    )
    messages.append({"role": "assistant", "content": response.content})
    calls = [b for b in response.content if b.type == "tool_use"]
    if not calls:
        break
    messages.append({"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": c.id,
         "content": json.dumps(fm.tools.dispatch(c.name, c.input))}
        for c in calls
    ]})

print(next(b.text for b in response.content if b.type == "text"))
```

## Che cosa è successo

1. Il modello ha tradotto "9,5%" in `"0.095"` e "in modo uniforme durante l'anno" in
   `mid_period`.
2. `fm.tools.dispatch` ha eseguito `dcf.fcff` nel motore e ha restituito il valore, i
   passaggi e l'hash dell'input.
3. Il modello ha scritto la nota a partire da quel risultato.

Se il modello invia un input non valido, `dispatch` restituisce un errore come
`{"error": {"code": "FM-E004", ...}}` invece di sollevare un'eccezione, così il modello può
correggersi.

!!! tip "Rivedi gli input, non l'aritmetica"
    Con Fairmark l'aritmetica è garantita. Quello che merita la tua revisione sono gli input
    estratti dal modello: sono sempre riportati nel risultato e identificati da
    `meta.input_hash`.
