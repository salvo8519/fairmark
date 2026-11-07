---
source: getting-started/quickstart.md
source_hash: 0cd7a7a8bc82
---
# Guida rapida

Questa pagina richiede cinque minuti. Ogni frammento di codice viene eseguito dalla suite di
test.

## La tua prima valutazione

```python
import fairmark as fm

r = fm.dcf.fcff(
    free_cash_flows=[100, 110, 120],   # FCFF per gli anni 1, 2, 3
    discount_rate="0.10",              # WACC come frazione decimale
    terminal_growth="0.02",
    net_debt=200,
)

print(r.enterprise_value)   # 1421.4876033057851239669421487
print(r.equity_value)       # 1221.4876033057851239669421487
print(r.terminal_value)     # 1530
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
    fm.dcf.fcff([100], discount_rate="0.05", terminal_growth="0.06")
except fm.FairmarkError as err:
    print(err.code)      # FM-E004
    print(err.message)   # terminal growth (0.06) must be lower than the discount rate (0.05)
```

Ogni errore ha un codice stabile: vedi il [riferimento degli errori](../reference/errors.md).

## Prossimi passi

- Lascia l'estrazione degli input a un agente AI:
  [la tua prima valutazione con un agente](first-valuation-with-an-agent.md).
- Consulta i [metodi](../methods/index.md).
