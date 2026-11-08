---
source: methods/statements-reorganise.md
source_hash: c53c9eb81ecb
---
# Bilancio riclassificato (`statements.reorganise`)

Riclassifica il bilancio di uno o più esercizi distinguendo ciò che l'impresa impiega nella
gestione (area operativa), ciò che detiene fuori dalla gestione (attività non operative) e come
è finanziata. Ogni esercizio è fornito come aggregati degli schemi civilistici (art. 2424 e 2425
del Codice civile, bilancio in forma ordinaria o in forma abbreviata, art. 2435-bis) oppure come
totali già riclassificati, per esempio da una banca. Il risultato ha le forme che
[`dcf.fcff`](dcf-fcff.md) e i gruppi `ratios.*` accettano senza modifiche.

Questa versione restituisce il **reorganised balance sheet** (stato patrimoniale riclassificato
in area operativa, non operativa e finanziaria): capitale investito,
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

Alcune voci sono considerate pari a zero quando non sono fornite, così che gli schemi
civilistici siano utilizzabili come depositati. Ciascuna è elencata in `meta.defaults_applied`
con il suo pointer e il valore `"0"`, ed è indicata nelle convenzioni:

- la voce A dell'attivo, crediti verso soci per versamenti ancora dovuti (decisione D1 del
  2026-11-08): lo stesso zero è dedotto dal patrimonio netto e dal totale attivo del blocco
  degli importi correnti e non correnti;
- una parte "di cui" (decisione D2): voce A o voce D dell'attivo compresa nei crediti, voce E
  del passivo compresa nei debiti, parte in derivati dei fondi per rischi e oneri, anche quando
  l'aggregato che la contiene non è fornito; nel bilancio in forma ordinaria ciascuna è una
  voce separata;
- la voce separata D o E, quando la voce D dell'attivo è fornita all'interno di C.II o la voce E
  del passivo all'interno di D (bilancio in forma abbreviata, art. 2435-bis).

Una relazione non è mai verificata su una voce considerata pari a zero: si veda
[Verifiche](#verifiche).

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
- operating provisions = fondi per rischi e oneri B e trattamento di fine rapporto C
  classificati come operativi;
- patrimonio netto = voce A del passivo − crediti verso soci per versamenti ancora dovuti (zero
  se non forniti) − dividendi deliberati;
- debiti finanziari lordi = debiti finanziari + debiti verso soci per finanziamenti classificati
  come debito finanziario; disponibilità liquide = C.IV − liquidità operativa; altre voci,
  ciascuna con il suo segno: C.III, crediti finanziari, B.III classificate come attività
  finanziaria (negative), derivati passivi, ratei per interessi e debiti commerciali con
  dilazione (positive);
- excess cash (liquidità eccedente) = C.IV − liquidità operativa, le disponibilità liquide della
  posizione finanziaria netta.

Derivati (decisione D3): i derivati passivi compresi nei fondi per rischi e oneri B classificati
come `financial_liability` sono non correnti nella posizione finanziaria netta. I derivati attivi
non sono forniti separatamente: quelli compresi in B.III (B.III.4) seguono la classificazione di
B.III e sono non correnti; quelli compresi in C.III (C.III.5) sono attività finanziarie correnti
della posizione finanziaria netta.

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
| Attivo A | `subscribed_capital_unpaid` (o compresa in C.II); zero se non fornita | dedotta dal patrimonio netto | fissa |
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
| B, derivati | `provisions.of_which_derivative_liabilities`; assente se non fornita | PFN (non corrente) oppure operating provisions | classificazione |
| C | `employee_severance_indemnity` | operating provisions | predefinita |
| D, debiti finanziari | `payables.financial_debt` | PFN, debiti finanziari lordi | fissa |
| D, debiti verso soci per finanziamenti | `payables.shareholder_loans` | debiti finanziari lordi oppure quasi-equity | classificazione |
| D, resto | `payables` al netto delle parti sopra (`trade`) | operating working capital | predefinita |
| E | `accruals_and_deferred_income` (o compresa in D) | operating working capital | predefinita |

I totali `fixed_assets` (B), `current_assets` (C), `total_assets` e
`total_liabilities_and_equity` sono facoltativi e verificati; quando sono forniti, il blocco degli
importi correnti e non correnti li usa al posto delle componenti mancanti.

### Stato patrimoniale riclassificato (Appendice B.1)

`operating_fixed_assets`, `operating_working_capital` (`total` oppure `trade_receivables`,
`inventories`, `other_operating_assets`, `trade_payables`, `other_operating_liabilities`),
`operating_provisions`, `invested_capital`, `non_operating_assets` (importi etichettati),
`total_funds_invested`, `equity`, `quasi_equity_items`, `net_financial_position` (`total`,
`position` nel tipo di SPEC-0007, `current`, `non_current`), `debt_like_items` (il tipo di
`dcf.fcff`, con `measurement_basis`) e `total_sources`. Ogni totale che la sua formula consente
è derivato da ciò che è fornito; un totale fornito è verificato rispetto al valore derivato. Il
totale della posizione finanziaria netta è, in quest'ordine, il totale fornito, il totale di
`position`, oppure `current` + `non_current`; in quest'ultimo caso la relazione sulle scadenze
non è verificata, perché confronterebbe il totale con sé stesso. Classificazioni e rettifiche
non si applicano a un esercizio riclassificato: fornire i totali già rettificati.

Il blocco degli importi correnti e non correnti e la composizione della posizione finanziaria
netta richiedono la forma civilistica: in un esercizio riclassificato sono elencati in
`not_computed` senza un pointer da fornire, e una convenzione lo indica. Le rimanenze sono tratte
da `operating_working_capital.inventories`.

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
dell'esercizio. Una parte dei debiti verso soci per finanziamenti classificata come debito
finanziario può indicare il suo `beyond_one_year`, non superiore alla parte; una parte
`quasi_equity` non può (`FM-E001`), perché non ha scadenza nella posizione finanziaria netta.
Quando ogni parte è debito finanziario con il suo `beyond_one_year`, la somma delle parti
esigibili oltre l'esercizio successivo è uguale a quella dei debiti verso soci; altrimenti la
relazione non è verificata, ma una somma già superiore è rifiutata (`FM-E026`).

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
| `current_non_current` | `fixed_assets`, `current_assets`, `inventories`, `cash`, `total_assets` (al netto della voce A), `current_liabilities`, `non_current_liabilities`; dai totali B, C e dal totale attivo quando forniti |
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

Ogni valore predefinito è elencato in `meta.defaults_applied` e in `explain.conventions`, in
quest'ordine:

1. ogni aggregato civilistico fornito la cui destinazione è una predefinita dell'Appendice A
   (B.I, B.II, C.II, C.III, C.IV, voce D dell'attivo, fondi per rischi e oneri B, trattamento di
   fine rapporto C, debiti D, voce E del passivo): `field` è il pointer della voce, `value` la
   destinazione, e la convenzione riporta "assigned to". Un aggregato è fornito quando è fornito
   il suo importo: un nodo che ne contiene solo una parte (per esempio `receivables.trade`) non
   basta;
2. `treatments.write_downs_of_receivables`, `investing` se non fornita;
3. ogni voce considerata pari a zero perché non fornita ([Calcolo sui dati
   disponibili](#calcolo-sui-dati-disponibili)): `value` è `"0"` e la convenzione riporta "not
   given, set to".

Le destinazioni fisse non sono valori predefiniti e non sono elencate. Le destinazioni
predefinite del conto economico (A5, B14, B10 d) sono elencate dalla versione che lo
riclassifica.

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
| Voce A non fornita | zero, lo stesso importo per patrimonio netto e totale attivo | predefinito, elencato (D1) |
| Parti "di cui" non fornite | assenti nel loro aggregato, anche quando l'aggregato non è fornito | predefinito, elencato (D2) |
| Derivati | passivi classificati come finanziari: non correnti nella posizione finanziaria netta; attivi secondo B.III (non correnti) o C.III (correnti) | fisso, indicato (D3) |

## Verifiche

Ogni relazione è verificata in modo esatto quando entrambi i lati sono forniti, e `FM-E026`
riporta i due importi, la differenza, i pointer e la correzione quando differiscono. Un lato è
costruito solo con voci fornite dall'utente: una voce considerata pari a zero non entra mai in
una verifica, e il messaggio e i `pointers` di una relazione verificata indicano solo voci
fornite. Senza totale attivo, per esempio, il totale attivo ricostruito come A + B + C + D è
confrontato solo quando sono fornite la voce A e la voce separata D.

Forma civilistica: B.I e B.II nette = lordo − fondo; totale B, totale C, totale del patrimonio
netto, totale attivo e totale passivo e patrimonio netto rispetto alle loro voci; totale attivo
= totale passivo e patrimonio netto; parti esigibili oltre l'esercizio e parti "di cui" entro il
loro aggregato; utile di A.IX = voce 21; ripartizione di un override = aggregato dell'esercizio;
le scadenze di una ripartizione dei debiti verso soci per finanziamenti (si veda
[Classificazioni](#classificazioni)); i totali del conto economico (A, B, B10, A − B, C, D,
risultato prima delle imposte, voce 21). Forma riclassificata: operating working capital,
capitale investito, total funds invested, posizione finanziaria netta (rispetto alla sua
composizione e alle sue scadenze) e total sources rispetto alle loro componenti, e total funds
invested = total sources.

Nella forma civilistica total funds invested è uguale alle fonti ogni volta che tutte le voci
sono fornite, perché l'identità discende dalle relazioni precedenti. Quando differiscono, gli
input forniti non quadrano una volta considerate pari a zero le voci non fornite: il risultato è
`FM-E026`, che indica quelle voci e chiede quella che non è zero, oppure il totale attivo e il
totale passivo e patrimonio netto, così che i totali siano confrontati. Con tutte le voci
fornite, una differenza è un difetto (`FM-E012`).

Per i bilanci depositati in unità di euro, fornire la riserva da arrotondamento nelle altre
riserve (A.VI), in modo che i totali quadrino esattamente.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Entrambe le forme in una voce; date non distanti un esercizio o dalla più recente; un importo negativo dove lo schema non lo prevede; un override di un altro aggregato; `beyond_one_year` su una parte `quasi_equity` di una ripartizione; una lista `amounts` di lunghezza diversa; un'etichetta o un motivo vuoti; una rettifica su un esercizio riclassificato; una rettifica del conto economico in questa versione; un campo o un valore sconosciuti |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |
| `FM-E009` | Una data inesistente o fuori dall'intervallo 1900-2199 |
| `FM-E012` | Difetto: total funds invested diverso dalle fonti di un esercizio civilistico con tutte le voci fornite |
| `FM-E026` | Una relazione del bilancio non quadra; total funds invested di un esercizio civilistico diverso dalle fonti una volta considerate pari a zero le voci non fornite; oppure una rettifica supera la sua voce di origine |

## Limiti e note di prassi

- **Imposte anticipate** (C.II 5-ter) restano nei crediti, e quindi nell'operating working
  capital. Nella prassi sono spesso escluse dal capitale circolante (Borsa Italiana, *Guida alla
  Valutazione*); per escluderle, spostarle con una rettifica `non_operating_asset` dei
  `receivables` e indicarne il motivo.
- **Crediti delle immobilizzazioni finanziarie** (B.III.2) seguono la classificazione di B.III e
  sono sempre non correnti nel blocco degli importi correnti e non correnti, anche per la parte
  esigibile entro l'esercizio successivo che l'art. 2424 chiede di indicare.
- **Esercizi comparativi anteriori al 2016.** Prima del D.Lgs. 139/2015 le azioni proprie erano
  iscritte nell'attivo (B.III.4 o C.III.5) con una riserva nel patrimonio netto; dal 2016 sono
  dedotte dal patrimonio netto tramite la riserva negativa per azioni proprie in portafoglio
  (A.X). Fornire un esercizio anteriore al 2016 come riesposto nella colonna comparativa del
  bilancio successivo, oppure dedurre le azioni proprie dal loro aggregato dell'attivo e
  indicarle in A.X, così che gli esercizi siano confrontabili.
- **Trattamento di fine rapporto** (TFR, voce C del passivo) è un operating provision come
  predefinito, al pari dei fondi per rischi e oneri B; quando l'analista lo considera un debito,
  una rettifica `debt_like_provision` lo sposta, con il suo motivo, tra i debt-like items del
  bridge.
- **Debiti verso imprese del gruppo** (D.8-D.11: verso imprese controllate, collegate,
  controllanti e sottoposte al controllo delle controllanti) restano nel resto dei debiti D,
  nell'operating working capital: questa versione non ne separa la parte finanziaria (decisione
  D6 del 2026-11-08; issue separata). Quando una loro parte è finanziamento, fornirla in
  `payables.financial_debt`, oppure spostarla con una rettifica `non_current_trade_payables`, e
  indicarne il motivo.

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
- `python/tests/test_reference_statements.py`: il motore rispetto a quel riferimento su 400
  chiamate casuali con seme fisso nelle due forme, per ogni output, verifica, valore predefinito
  e voce di `not_computed`, comprese le voci considerate pari a zero, le ripartizioni dei debiti
  verso soci per finanziamenti con scadenze e le posizioni finanziarie nette riclassificate
  fornite per parti.

## Fonti

- Codice civile, art. 2423-ter, 2424, 2425, 2435-bis e 2435-ter (schemi e dati comparativi).
- OIC 12 (composizione e schemi del bilancio d'esercizio); D.Lgs. 139/2015 (azioni proprie
  dedotte dal patrimonio netto dal 2016).
- Borsa Italiana, *Guida alla Valutazione* (trattamento delle imposte anticipate nella prassi).
- ESMA32-382-1138, orientamento 39, e Consob, Richiamo di attenzione n. 5/21 (indebitamento
  finanziario netto).
- Koller, Goedhart, Wessels, *Valuation*, "Reorganizing the financial statements".
- SPEC-0005, le sue decisioni da 1 a 18 e le decisioni del maintainer da D1 a D6 del
  2026-11-08; SPEC-0007 (posizione finanziaria netta); SPEC-0001 Decision 21 (voci del bridge e
  `MeasurementBasis`).
