---
source: getting-started/quickstart.md
source_hash: dddd00a55c25
---
# Guida rapida

Questa pagina richiede cinque minuti. Ogni frammento di codice viene eseguito dalla suite di
test.

## La tua prima valutazione

```python
import fairmark as fm

r = fm.dcf.fcff(
    [fm.dcf.given([100, 110, 120])],   # un solo stage: FCFF per gli anni 1, 2, 3
    cash_flow_terms="nominal",         # flussi e tassi negli stessi termini
    discount_rate_terms="nominal",
    wacc="0.10",                       # WACC come frazione decimale
    timing="end_of_period",
    terminal=fm.dcf.gordon("0.02", first_flow="last_flow_grown"),
    net_financial_position={"gross_debt": 200, "cash": 0},
)

print(r.value_of_operations)          # 1421.4876033057851239669421487
print(r.equity_value)                 # 1221.4876033057851239669421487
print(r.terminal["terminal_value"])   # 1530
```

I valori sono `Decimal` di Python. Passa i numeri come stringhe o `Decimal` per mantenerli
esatti; i float sono accettati e convertiti tramite la loro rappresentazione più breve
(`0.1` → `"0.1"`).

## Come è stato calcolato

```python
for step in r.steps:
    print(f"{step.label:<32} {step.formula:<40} {step.value}")

print(r.conventions)
```

`r.explain_markdown()` restituisce le stesse informazioni come tabella da incollare in una
relazione.

## Riprodurre un risultato in seguito

```python
print(r.meta["input_hash"])       # SHA-256 dell'input canonico
print(r.meta["engine_version"])
```

Stesso hash e stesse versioni significano lo stesso risultato. Da riga di comando,
`fairmark verify result.json --input inputs.json` esegue il controllo per te.

## Gestire gli errori

```python
try:
    fm.dcf.fcff(
        [fm.dcf.given([100])],
        cash_flow_terms="nominal",
        discount_rate_terms="nominal",
        wacc="0.05",
        timing="end_of_period",
        terminal=fm.dcf.gordon("0.06", first_flow="last_flow_grown"),
    )
except fm.FairmarkError as err:
    print(err.code)      # FM-E004
    print(err.message)   # `/terminal/tv.gordon/growth` (0.06) must be lower than ... `/wacc` (0.05) ...
```

Ogni errore ha un codice stabile: vedi il [riferimento degli errori](../reference/errors.md).

## Prossimi passi

- Lascia l'estrazione degli input a un agente AI:
  [la tua prima valutazione con un agente](first-valuation-with-an-agent.md).
- Consulta i [metodi](../methods/index.md).
