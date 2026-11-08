---
source: methods/tv-compare.md
source_hash: 1994ab9f43c6
---
# Confronto tra tecniche di valore terminale (`tv.compare`)

Esegue più tecniche di valore terminale su un unico insieme coerente di input e restituisce,
affiancati, i valori terminali all'orizzonte N con le ipotesi implicite di ciascuno. La
conclusione di una valutazione dipende spesso più dalla tecnica che da un singolo input; questo
metodo mostra di quanto. La coerenza è garantita dalla struttura: il tasso della fase
terminale, il limite alla crescita e l'approccio (asset side o equity side) sono forniti una
volta e valgono per ogni tecnica.

## Come funziona

Ogni tecnica si scrive `{"<id del metodo>": {termini}}`, dove i termini sono gli input di quel
metodo senza `rate` e `growth_cap`:

| Tecnica | Termini | Input comuni utilizzati |
| --- | --- | --- |
| [`tv.gordon`](tv-gordon.md) | `growth`, `next_flow` oppure `last_flow` | tasso, limite alla crescita |
| [`tv.value_driver`](tv-value-driver.md) | `nopat_next`, `growth`, `ronic` | tasso, limite alla crescita |
| [`tv.h_model`](tv-h-model.md) | `last_flow`, `short_term_growth`, `long_term_growth`, `half_life` | tasso, limite alla crescita |
| [`tv.exit_multiple`](tv-exit-multiple.md) | `multiple`, `metric`, `metric_value`, `next_flow` facoltativo | tasso e limite alla crescita quando è fornito `next_flow`; la grandezza deve essere sull'approccio dichiarato |
| [`tv.limited_horizon`](tv-limited-horizon.md) | `flow`, `periods`, `growth` e `residual_value` facoltativi | tasso, limite alla crescita |
| [`tv.normalised_income`](tv-normalised-income.md) | `income`, `capitalisation_rate`, `growth` facoltativo | limite alla crescita; il tasso di capitalizzazione resta il proprio |
| [`tv.liquidation`](tv-liquidation.md) | `assets`, `liabilities`, `liquidation_costs`, `tax_on_gains` | nessuno; le passività elencate devono essere coerenti con l'approccio dichiarato |

Ogni tecnica è calcolata dal proprio metodo, quindi una riga del confronto coincide con il
risultato di quel metodo eseguito da solo. Passaggi e convenzioni di ciascuna tecnica compaiono
in `explain`, preceduti dalla posizione e dall'id, ad esempio
`[1] tv.gordon: Terminal value at N`. La prima tecnica che fallisce restituisce il proprio
errore; non ci sono risultati parziali.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

r = fm.tv.compare(
    "0.10",
    "enterprise",
    [
        {"tv.gordon": {"growth": "0.02", "last_flow": 100}},
        {"tv.value_driver": {"nopat_next": 150, "growth": "0.02", "ronic": "0.12"}},
        {"tv.exit_multiple": {"multiple": "7.5", "metric": "ebitda", "metric_value": 180,
                              "next_flow": 95}},
    ],
    growth_cap="0.03",
)
values = [row["terminal_value"] for row in r.results]
assert values[0] == Decimal("1275")
assert values[2] == Decimal("1350")
assert round(r.results[2]["output"]["implied_growth"], 6) == Decimal("0.029630")

try:  # il P/E è una grandezza equity side: mai in un confronto asset side
    fm.tv.compare("0.10", "enterprise", [
        {"tv.exit_multiple": {"multiple": 12, "metric": "earnings", "metric_value": 50}},
    ])
except fm.FairmarkError as err:
    assert err.code == "FM-E019"
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `rate` | Tasso di attualizzazione della fase terminale, frazione decimale per periodo, comune | obbligatorio |
| `basis` | `enterprise` oppure `equity`, comune a tutti i valori terminali confrontati | obbligatorio |
| `growth_cap` | Limite alla crescita perpetua, frazione decimale per periodo, comune | nessuno, riportato come assente |
| `techniques` | Elenco di `{"<id del metodo>": {termini}}`, almeno uno; una tecnica può comparire più volte | obbligatorio |

## Output

| Campo | Significato |
| --- | --- |
| `basis` | L'approccio dichiarato |
| `results` | Una riga per tecnica, nell'ordine richiesto: `method`, `terminal_value` e `output`, l'output della tecnica con le sue ipotesi implicite |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Approccio | enterprise oppure equity, dichiarato una volta | nessuno: va fornito |
| Tasso | un solo tasso per ogni tecnica che attualizza; il tasso di capitalizzazione di `tv.normalised_income` resta il proprio | nessuno: va fornito |
| Limite alla crescita | un solo valore per ogni tecnica basata sulla crescita, oppure nessuno | nessuno, riportato come assente |

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | L'elenco delle tecniche è vuoto, l'id di una tecnica è sconosciuto, oppure un campo manca o è sconosciuto |
| `FM-E019` | Un multiplo di uscita è sull'approccio diverso da quello dichiarato |
| Qualsiasi codice di una tecnica | La prima tecnica che fallisce restituisce il proprio errore (ad esempio `FM-E004`, `FM-E018`) |

## Precisione

Ogni riga ha la precisione del proprio metodo; il confronto non aggiunge calcoli.

## Verificato con

- `tests/golden/tv.compare/`: sette tecniche su un unico tasso, approccio e limite, ciascun
  valore atteso calcolato separatamente da uno script indipendente a 50 cifre
  (`scripts/reference_tv.py`).
- Un test Python verifica che ogni riga coincida con il metodo eseguito da solo.
