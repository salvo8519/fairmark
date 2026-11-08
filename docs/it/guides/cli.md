---
source: guides/cli.md
source_hash: b05264e6d6e9
---
# Riga di comando

Il binario `fairmark` legge JSON e scrive JSON. Gli errori vanno su stderr come JSON, con un
codice di uscita diverso da zero.

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
