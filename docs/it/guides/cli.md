---
source: guides/cli.md
source_hash: d1427819984b
---
# Riga di comando

Il binario `fairmark` legge JSON e scrive JSON. Gli errori vanno su stderr come JSON, con un
codice di uscita diverso da zero.

```bash
fairmark methods                                   # elenca i metodi
fairmark schema dcf.fcff                           # JSON Schema degli input
fairmark run dcf.fcff --input company.json         # risultato completo: value, explain, meta
fairmark run dcf.fcff --input company.json --value-only
cat company.json | fairmark run dcf.fcff           # funziona anche da stdin
fairmark verify result.json --input company.json   # riproduce e confronta
```

Esempio di input:

```json
{
  "free_cash_flows": ["1200000", "1350000", "1500000", "1600000", "1700000"],
  "discount_rate": "0.095",
  "terminal_growth": "0.02",
  "timing": "mid_period",
  "net_debt": "3100000"
}
```
