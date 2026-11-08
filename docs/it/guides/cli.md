---
source: guides/cli.md
source_hash: 52c142b298d4
---
# Riga di comando

Il binario `fairmark` legge JSON e scrive JSON. Gli errori vanno su stderr come JSON, con un
codice di uscita diverso da zero.

Codici di uscita:

| Codice | Significato |
| --- | --- |
| `0` | Successo: il risultato è stato scritto su stdout, oppure il lettore ha chiuso la pipe in anticipo |
| `1` | Errore: il report JSON su stderr riporta il codice e il messaggio; `verify` esce con `1` anche quando il risultato non coincide |
| `2` | Riga di comando non valida (sottocomando od opzione sconosciuti); il messaggio è quello del parser degli argomenti |

Quando il programma che legge l'output si ferma prima, come in `fairmark methods | head -1`,
`fairmark` smette di scrivere ed esce con `0`, senza nulla su stderr. È il lettore ad aver
scelto di fermarsi, quindi non è un errore, e gli script con `set -o pipefail` non falliscono
per questo. Il comportamento è diverso da quello di strumenti C come `cat`, che vengono
terminati dal segnale `SIGPIPE` e nella shell riportano `141`.

```bash
fairmark methods                                   # elenca i metodi, con versione e stato
fairmark schema dcf.fcff                           # JSON Schema degli input
fairmark run dcf.fcff --input company.json         # risultato completo: value, explain, meta
fairmark run dcf.fcff --input company.json --value-only
cat company.json | fairmark run dcf.fcff           # funziona anche da stdin
fairmark verify result.json --input company.json   # riproduce e confronta
```

Esempio di input:

```json
{
  "stages": [
    {"periods": 5, "projection": "given",
     "flows": ["1200000", "1350000", "1500000", "1600000", "1700000"]}
  ],
  "cash_flow_terms": "nominal",
  "discount_rate_terms": "nominal",
  "wacc": "0.095",
  "timing": "mid_period",
  "terminal_discount_time": "horizon",
  "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
  "net_financial_position": {"gross_debt": "3100000", "cash": "0"}
}
```
