---
source: methods/statements-reorganise.md
source_hash: a96a77992e30
---
# Bilancio riclassificato (`statements.reorganise`)

Riclassifica il bilancio di uno o più esercizi distinguendo ciò che l'impresa impiega nella
gestione (area operativa), ciò che detiene fuori dalla gestione (attività non operative) e come
è finanziata. Ogni esercizio è fornito come aggregati degli schemi civilistici (art. 2424 e 2425
del Codice civile, bilancio in forma ordinaria o in forma abbreviata, art. 2435-bis) oppure come
totali già riclassificati, per esempio da una banca. Il risultato ha le forme che
[`dcf.fcff`](dcf-fcff.md) e i gruppi `ratios.*` accettano senza modifiche.

Questa versione restituisce lo **stato patrimoniale riclassificato**: capitale investito,
attività non operative, patrimonio netto, quasi-equity items, posizione finanziaria netta con
composizione e scadenze, debt-like items e importi correnti e non correnti. Le voci del conto
economico sono lette, validate e verificate rispetto ai loro totali; la loro riclassificazione
non è ancora restituita (si veda la
[roadmap](https://salvo8519.github.io/fairmark/project/roadmap/), in inglese).

## Calcolo sui dati disponibili

Ogni input è facoltativo. Un output è calcolato quando sono forniti gli input di cui ha bisogno;
altrimenti è `null` ed è elencato in `not_computed` con i JSON pointer degli input mancanti. Un
input mancante non è mai considerato zero e non è mai un errore. Un agente che legge pochi
totali da un testo può passarli, ricevere ciò che consentono e chiedere all'utente soltanto i
pointer elencati.

Due letture rendono utilizzabili gli schemi civilistici così come depositati:

- una parte "di cui" non fornita (voci A o D dell'attivo comprese nei crediti, voce E del
  passivo compresa nei debiti, parte in derivati dei fondi per rischi e oneri) è considerata
  assente all'interno del suo aggregato, e le convenzioni la elencano: nel bilancio in forma
  ordinaria ciascuna è una voce separata;
- le voci A, D ed E fornite all'interno di C.II o di D (bilancio in forma abbreviata) valgono
  zero come voci separate.

I debiti finanziari e i debiti verso soci per finanziamenti compresi nei debiti sono necessari
per la posizione finanziaria netta e per l'operating working capital (Decision 4): senza di essi
nessuno dei due è calcolato.

## Formula

$$
\text{capitale investito} = \text{operating fixed assets} + \text{operating working capital} - \text{operating provisions}
$$

$$
\text{total funds invested} = \text{capitale investito} + \sum \text{attività non operative}
$$

$$
= \text{patrimonio netto} + \sum \text{quasi-equity items} + \text{PFN} + \sum \text{debt-like items}
$$

$$
\text{PFN} = \text{debiti finanziari lordi} - \text{disponibilità liquide} + \sum \text{altre voci}
$$

con, per un esercizio in forma civilistica (Appendice B.1 di SPEC-0005):

- operating fixed assets (immobilizzazioni operative) = B.I + B.II + B.III classificate come
  operative − parti spostate tra le attività non operative;
- operating working capital (capitale circolante netto operativo) = C.I + C.II (parte operativa)
  + voce D dell'attivo + liquidità operativa − debiti D (parte operativa) − voce E del passivo
  (parte operativa);
- operating provisions (fondi operativi) = fondi per rischi e oneri B e trattamento di fine
  rapporto C classificati come operativi;
- patrimonio netto = voce A del passivo − crediti verso soci per versamenti ancora dovuti −
  dividendi deliberati;
- debiti finanziari lordi = debiti finanziari + debiti verso soci per finanziamenti classificati
  come debito finanziario; disponibilità liquide = C.IV − liquidità operativa; altre voci,
  ciascuna con il suo segno: C.III, crediti finanziari, B.III classificate come attività
  finanziaria (negative), derivati passivi, ratei per interessi e debiti commerciali con
  dilazione (positive).

Una posizione finanziaria netta positiva è indebitamento netto. Il suo totale è calcolato dalla
funzione degli indici (SPEC-0007), quindi coincide con il totale dell'oggetto restituito.

## Esempio

```python
from decimal import Decimal

import fairmark as fm

r = fm.statements.reorganise(
    [
        {
            "label": "2024",
            "date": "2024-12-31",
            "statutory": {
                "balance_sheet": {
                    "subscribed_capital_unpaid": "0",
                    "intangible_assets": {"total": "120000"},
                    "tangible_assets": {"total": "900000"},
                    "financial_fixed_assets": "150000",
                    "inventories": "400000",
                    "receivables": {"total": "950000", "beyond_one_year": "50000", "trade": "800000"},
                    "current_financial_assets": "60000",
                    "cash": "240000",
                    "prepayments_and_accrued_income": {"total": "30000"},
                    "equity": {"total": "900000"},
                    "provisions": {"total": "70000", "of_which_derivative_liabilities": "10000"},
                    "employee_severance_indemnity": {"total": "180000"},
                    "payables": {
                        "total": "1650000",
                        "beyond_one_year": "600000",
                        "financial_debt": {"total": "900000", "beyond_one_year": "550000"},
                        "shareholder_loans": {"total": "100000", "beyond_one_year": "50000"},
                        "trade": "500000",
                    },
                    "accruals_and_deferred_income": {"total": "50000"},
                }
            },
        }
    ],
    treatments={
        "financial_fixed_assets": "non_operating",
        "shareholder_loans": "financial_debt",
        "derivative_liabilities": "financial_liability",
    },
)
bs = r.years[0]["balance_sheet"]
assert bs["operating"]["invested_capital"] == Decimal("1460000")  # 1020000 + 680000 − 240000
assert bs["non_operating_assets_total"] == Decimal("150000")
nfp = bs["sources"]["net_financial_position"]
assert nfp["total"] == Decimal("710000")  # 1000000 − 240000 − 60000 + 10000
assert (nfp["current"], nfp["non_current"]) == (Decimal("100000"), Decimal("610000"))
assert bs["total_funds_invested"] == bs["sources"]["total_sources"] == Decimal("1610000")
```

## Input

| Campo | Significato | Predefinito |
| --- | --- | --- |
| `years` | Uno o più esercizi, dal più vecchio | obbligatorio |
| `years[].label`, `years[].date` | Nome dell'esercizio; data di chiusura (ISO 8601) | nessuno; ordine assunto come fornito, riportato |
| `years[].statutory` | `layout` (riportato), `balance_sheet`, `income_statement` | questo oppure `reclassified` |
| `years[].reclassified` | `balance_sheet`, `income_statement` con i totali dell'Appendice B | questo oppure `statutory` |
| `years[].treatment_overrides` | Un'altra destinazione, o una ripartizione per importi, di B.III o dei debiti verso soci per finanziamenti per quell'esercizio | nessuno |
| `treatments` | Destinazioni indicate una volta per la chiamata | si veda [Classificazioni](#classificazioni) |
| `adjustments` | Rettifiche etichettate dell'Appendice C, un importo per esercizio | nessuna |

Gli importi sono nelle unità di valuta del bilancio, mai scalati, come esposti negli schemi
civilistici: zero o più, salvo le voci con segno (patrimonio netto A.VI-A.IX e il suo totale,
A.X zero o negativa; A2 e A3, B11, C17-bis, voci 20 e 21 e i totali che le contengono; i totali
della forma riclassificata definiti come differenze).

### Stato patrimoniale civilistico (Appendice A.1)

| Riferimento | Campo | Destinazione | Scelta |
| --- | --- | --- | --- |
| Attivo A | `subscribed_capital_unpaid` (o compresa in C.II) | dedotta dal patrimonio netto | fissa |
| B.I | `intangible_assets` (`total`, oppure `gross` e `accumulated`) | operating fixed assets | predefinita |
| B.II | `tangible_assets` (`total`, oppure `gross` e `accumulated`) | operating fixed assets | predefinita |
| B.III | `financial_fixed_assets` | operativa, non operativa o attività finanziaria | classificazione |
| C.I | `inventories` | operating working capital | fissa |
| C.II | `receivables` (`beyond_one_year`, `trade` e le parti comprese) | operating working capital | predefinita |
| C.III | `current_financial_assets` | PFN, altra attività finanziaria | predefinita |
| C.IV | `cash` | PFN, disponibilità liquide | predefinita |
| Attivo D | `prepayments_and_accrued_income` (o compresa in C.II) | operating working capital | predefinita |
| Passivo A | `equity` (`total` oppure A.I-A.X) | patrimonio netto | fissa |
| B | `provisions` (`of_which_derivative_liabilities`, `beyond_one_year`) | operating provisions | predefinita |
| B, derivati | `provisions.of_which_derivative_liabilities` | PFN oppure operating provisions | classificazione |
| C | `employee_severance_indemnity` | operating provisions | predefinita |
| D, debiti finanziari | `payables.financial_debt` | PFN, debiti finanziari lordi | fissa |
| D, debiti verso soci per finanziamenti | `payables.shareholder_loans` | debiti finanziari lordi oppure quasi-equity | classificazione |
| D, resto | `payables` al netto delle parti sopra (`trade`) | operating working capital | predefinita |
| E | `accruals_and_deferred_income` (o compresa in D) | operating working capital | predefinita |

I totali `fixed_assets` (B), `current_assets` (C), `total_assets` e
`total_liabilities_and_equity` sono facoltativi e verificati.

### Stato patrimoniale riclassificato (Appendice B.1)

`operating_fixed_assets`, `operating_working_capital` (`total` oppure `trade_receivables`,
`inventories`, `other_operating_assets`, `trade_payables`, `other_operating_liabilities`),
`operating_provisions`, `invested_capital`, `non_operating_assets` (importi etichettati),
`total_funds_invested`, `equity`, `quasi_equity_items`, `net_financial_position` (`total`,
`position` nel tipo di SPEC-0007, `current`, `non_current`), `debt_like_items` (il tipo di
`dcf.fcff`, con `measurement_basis`) e `total_sources`. Ogni totale che la sua formula consente
è derivato da ciò che è fornito; un totale fornito è verificato rispetto al valore derivato.
Classificazioni e rettifiche non si applicano a un esercizio riclassificato: fornire i totali già
rettificati.

## Classificazioni

Indicate una volta in `treatments` e applicate a ogni esercizio in forma civilistica. Una
classificazione è necessaria solo quando il suo aggregato è fornito e diverso da zero; quando è
necessaria e manca, gli output che dipendono dall'aggregato non sono calcolati e `not_computed`
indica la classificazione e le destinazioni ammesse.

| Classificazione | Valori | Predefinito |
| --- | --- | --- |
| `financial_fixed_assets` | `operating`, `non_operating`, `financial_asset` | nessuno |
| `shareholder_loans` | `financial_debt`, `quasi_equity` | nessuno |
| `derivative_liabilities` | `financial_liability`, `operating_provision` | nessuno |
| `write_downs_of_receivables` | `investing`, `operating` (usata dal conto economico) | `investing` |

I `treatment_overrides` di un esercizio accettano solo `financial_fixed_assets` e
`shareholder_loans`, come `{"destination": ...}` oppure
`{"split": [{"destination", "amount", "label"}, ...]}` con parti che sommano all'aggregato
dell'esercizio; una parte dei debiti verso soci per finanziamenti classificata come debito
finanziario può indicare il suo `beyond_one_year`.

## Rettifiche

Ogni rettifica è indicata una volta con `kind`, `label`, `reason` (non vuota) e `amounts`, una
voce per esercizio nell'ordine di `years` (`null` dove non si applica, riportato). Un importo
superiore a quanto resta nella voce di origine è `FM-E026`.

| Tipo | Origine | Destinazione | Campi |
| --- | --- | --- | --- |
| `non_operating_asset` | B.I, B.II, C.II o C.III | attività non operative | `aggregate` |
| `financial_receivable` | C.II | PFN, altra attività finanziaria | `maturity` |
| `operating_cash` | disponibilità liquide della PFN | operating working capital | nessuno |
| `accrued_interest` | E | PFN, altra passività finanziaria | nessuno |
| `tax_instalments` | debiti D (resto) | debt-like items | `maturity` |
| `non_current_trade_payables` | debiti D (resto) | PFN, altra passività finanziaria | nessuno |
| `debt_like_provision` | B o C (operativi) | debt-like items | `aggregate` |
| `dividends_declared` | patrimonio netto | debt-like items | nessuno |

I tipi `lease`, `investing` e `non_recurring` rettificano il conto economico; questa versione li
rifiuta con `FM-E001` invece di ignorarli.

## Output

Per esercizio: `label`, `date`, `form`, `layout`, poi:

| Campo | Significato |
| --- | --- |
| `balance_sheet.operating` | `operating_fixed_assets`, `operating_working_capital`, `trade_working_capital`, `operating_provisions`, `invested_capital` |
| `balance_sheet.non_operating_assets` | Importi etichettati nel tipo di `dcf.fcff`, esclusi dal capitale investito; con il loro totale |
| `balance_sheet.total_funds_invested` | Capitale investito + attività non operative |
| `balance_sheet.sources` | `equity`, `quasi_equity_items`, `net_financial_position`, `debt_like_items` (al valore contabile), `total_sources` |
| `balance_sheet.sources.net_financial_position` | `position` (tipo di SPEC-0007), `total`, `current`, `non_current`, `composition` per origine |
| `balance_sheet.excess_cash` | C.IV al netto della liquidità operativa |
| `current_non_current` | `fixed_assets`, `current_assets`, `inventories`, `cash`, `total_assets`, `current_liabilities`, `non_current_liabilities` |
| `assignments` | Ogni aggregato fornito: riferimento, pointer, importo, destinazione, origine (`mapping` o `analyst`) |
| `adjustments_applied` | Ogni rettifica: tipo, etichetta, importo, motivo, origine, destinazione |
| `checks` | Ogni relazione per identificativo: `reconciled` o `not_checked`, con i suoi pointer |
| `not_computed` | Per blocco, ogni output mancante con i pointer degli input mancanti e, per una classificazione mancante, le destinazioni ammesse |

| Output | Alimenta |
| --- | --- |
| `net_financial_position.position` | `net_financial_position` di `dcf.fcff` e di `ratios.capital_structure` |
| `non_operating_assets` | `non_operating_assets` di `dcf.fcff` |
| `debt_like_items` | `debt_like_items` di `dcf.fcff` |
| `current_non_current` | `ratios.liquidity` e `ratios.capital_structure` |
| `invested_capital` | `invested_capital` di `ratios.profitability` |

## Valori predefiniti

Ogni valore predefinito è elencato in `meta.defaults_applied` e in `explain.conventions`:

- ogni aggregato civilistico fornito la cui destinazione è una predefinita dell'Appendice A
  (B.I, B.II, C.II, C.III, C.IV, voce D dell'attivo, fondi per rischi e oneri B, trattamento di
  fine rapporto C, debiti D, voce E del passivo, e A5 e B14 del conto economico): `field` è il
  pointer della voce, `value` la destinazione, e la convenzione riporta "assigned to";
- `treatments.write_downs_of_receivables`, `investing` se non fornita, e la voce B10 d quando è
  fornita con quel valore predefinito.

Le destinazioni fisse non sono valori predefiniti e non sono elencate.

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Predefinito |
| --- | --- | --- |
| Esercizi | dal più vecchio; date verificate quando le voci consecutive le hanno: stesso mese e giorno un anno solare dopo, 28 febbraio dopo 29 febbraio, oppure entrambe fine mese (28 febbraio 2023, 29 febbraio 2024) | ordine assunto e riportato quando manca una data |
| Input mancante | output nullo, elencato in `not_computed` | mai zero, mai un errore |
| Verifiche | esatte, solo quando entrambi i lati sono forniti | `not_checked` altrimenti |
| Schema | `ordinary`, `abridged`, `micro_entity` | riportato; nessun numero ne dipende |
| Fondi per rischi e oneri B, trattamento di fine rapporto C | operating provisions; debt-like items tramite rettifica | predefinito |
| Posizione finanziaria netta | C.III e tutta la C.IV all'interno; liquidità operativa esclusa tramite rettifica | predefinito |
| Voci del bridge | valore contabile, `book_value` | fisso |
| Correnti e non correnti | importi esigibili oltre l'esercizio successivo di C.II e D, letti come dodici mesi; fondi e trattamento di fine rapporto non correnti, ratei e risconti correnti, salvo che sia fornita la loro parte oltre l'esercizio; non modificati dalle rettifiche | predefinito, ciascuno riportato |

## Verifiche

Ogni relazione è verificata in modo esatto quando entrambi i lati sono forniti, e `FM-E026`
riporta i due importi, la differenza, i pointer e la correzione quando differiscono. Forma
civilistica: B.I e B.II nette = lordo − fondo; totale B, totale C, totale del patrimonio netto,
totale attivo e totale passivo e patrimonio netto rispetto alle loro voci; totale attivo =
totale passivo e patrimonio netto; parti esigibili oltre l'esercizio e parti "di cui" entro il
loro aggregato; utile di A.IX = voce 21; ripartizione di un override = aggregato dell'esercizio;
i totali del conto economico (A, B, B10, A − B, C, D, risultato prima delle imposte, voce 21).
Forma riclassificata: operating working capital, capitale investito, total funds invested,
posizione finanziaria netta (rispetto alla sua composizione e alle sue scadenze) e total sources
rispetto alle loro componenti, e total funds invested = total sources. Nella forma civilistica
l'ultima identità discende dalle altre; una differenza è un difetto (`FM-E012`).

Per i bilanci depositati in unità di euro, fornire la riserva da arrotondamento nelle altre
riserve (A.VI), in modo che i totali quadrino esattamente.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Entrambe le forme in una voce; date non distanti un esercizio o dalla più recente; un importo negativo dove lo schema non lo prevede; un override di un altro aggregato; una lista `amounts` di lunghezza diversa; un'etichetta o un motivo vuoti; una rettifica su un esercizio riclassificato; una rettifica del conto economico in questa versione; un campo o un valore sconosciuti |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |
| `FM-E009` | Una data inesistente o fuori dall'intervallo 1900-2199 |
| `FM-E012` | Difetto: total funds invested diverso dalle fonti di un esercizio civilistico |
| `FM-E026` | Una relazione del bilancio non quadra, oppure una rettifica supera la sua voce di origine |

## Accuratezza

Solo addizioni e sottrazioni di decimali, esatte; nessuna moltiplicazione, divisione o
arrotondamento.

## Verificato con

- Golden case in `tests/golden/statements.reorganise/` (lo stesso bilancio in forma ordinaria e
  abbreviata con output identici, una classificazione mancante, classificazioni e override su
  più esercizi, un caso per ogni tipo di rettifica dello stato patrimoniale, crediti verso soci
  per versamenti ancora dovuti, dividendi deliberati, date in anni bisestili e di fine mese, la
  forma riclassificata dello stesso bilancio, un caso `FM-E026` per relazione con il suo gemello
  non verificato, e gli errori di input), i cui valori attesi provengono da
  `scripts/reference_statements.py`, un riferimento decimale esatto a 50 cifre scritto dalle
  appendici di SPEC-0005 senza leggere il motore.
- `python/tests/test_reference_statements.py`: il motore rispetto a quel riferimento su 150
  chiamate casuali con seme fisso nelle due forme, per ogni output, verifica, valore predefinito
  e voce di `not_computed`.

## Fonti

- Codice civile, art. 2423-ter, 2424, 2425, 2435-bis e 2435-ter (schemi e dati comparativi).
- OIC 12 (composizione e schemi del bilancio d'esercizio).
- ESMA32-382-1138, orientamento 39, e Consob, Richiamo di attenzione n. 5/21 (indebitamento
  finanziario netto).
- Koller, Goedhart, Wessels, *Valuation*, "Reorganizing the financial statements".
- SPEC-0005 e le sue decisioni da 1 a 18; SPEC-0007 (posizione finanziaria netta); SPEC-0001
  Decision 21 (voci del bridge e `MeasurementBasis`).
