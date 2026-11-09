---
source: methods/statements-reorganise.md
source_hash: c3d59e08f998
---
# Bilancio riclassificato (`statements.reorganise`)

Riclassifica il bilancio di uno o più esercizi distinguendo ciò che l'impresa impiega nella
gestione (area operativa), ciò che detiene fuori dalla gestione (attività non operative) e come
è finanziata. Ogni esercizio è fornito come aggregati degli schemi civilistici (art. 2424 e 2425
del Codice civile, bilancio in forma ordinaria o in forma abbreviata, art. 2435-bis) oppure come
totali già riclassificati, per esempio da una banca. Il risultato ha le forme che
[`dcf.fcff`](dcf-fcff.md) e i gruppi `ratios.*` accettano senza modifiche.

Questa versione restituisce, per ogni esercizio, il **reorganised balance sheet** (stato
patrimoniale riclassificato in area operativa, non operativa e finanziaria: capitale investito,
attività non operative, patrimonio netto, quasi-equity items, posizione finanziaria netta con
composizione e scadenze, debt-like items e importi correnti e non correnti) e il **conto
economico** per natura dell'art. 2425 riclassificato (valore della produzione, value added,
margine operativo lordo, reddito operativo, investing result, reddito operativo aziendale,
componenti non ricorrenti, financing result, risultato prima delle imposte, imposte sul reddito e
utile netto), con la riconciliazione dei suoi due subtotali allineati a IFRS 18
con A − B. Le variazioni esatte tra esercizi consecutivi arrivano con una versione successiva
(si veda la [roadmap](https://salvo8519.github.io/fairmark/project/roadmap/), in inglese).

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
  del passivo all'interno di D (bilancio in forma abbreviata, art. 2435-bis; approvazione del
  maintainer del 2026-11-08).

La voce A e le parti "di cui" considerate pari a zero non entrano mai in una verifica; una parte
esposta all'interno di C.II o di D conta come fornita: si veda [Verifiche](#verifiche).

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
  come debito finanziario + debiti per leasing rappresentati con una rettifica `lease`
  (decisione P3 del 2026-11-09); disponibilità liquide = C.IV − liquidità operativa; altre voci,
  ciascuna con il suo segno: C.III, crediti finanziari, B.III classificate come attività
  finanziaria (negative), derivati passivi, ratei per interessi e debiti commerciali con
  dilazione (positive);
- excess cash (liquidità eccedente) = C.IV − liquidità operativa, le disponibilità liquide della
  posizione finanziaria netta.

**Rispetto alla tabella ESMA** (ESMA32-382-1138, orientamento 39; decisione P4 del
2026-11-09). La posizione finanziaria netta di Fairmark è la misura dell'analista di SPEC-0007,
non il "total financial indebtedness" dell'ESMA. Se ne discosta dove Fairmark deduce ciò che la
tabella ESMA non deduce: le attività finanziarie non correnti (B.III classificate come
`financial_asset`, i crediti finanziari esigibili oltre l'esercizio successivo, i derivati attivi
compresi in B.III) e i derivati attivi di copertura compresi in C.III, che l'ESMA esclude dalle
altre attività finanziarie correnti. La composizione elenca ciascuna di queste voci, quindi il
valore ESMA si ricostruisce togliendole. Per una vista ESMA (righe da A a M) si veda la
[roadmap](https://salvo8519.github.io/fairmark/project/roadmap/), in inglese.

Derivati (decisione D3): i derivati passivi compresi nei fondi per rischi e oneri B classificati
come `financial_liability` sono non correnti nella posizione finanziaria netta. I derivati attivi
non sono forniti separatamente: quelli compresi in B.III (B.III.4) seguono la classificazione di
B.III e sono non correnti; quelli compresi in C.III (C.III.5) sono attività finanziarie correnti
della posizione finanziaria netta.

Una posizione finanziaria netta positiva è indebitamento netto. Il suo totale è calcolato dalla
funzione degli indici (SPEC-0007), quindi coincide con il totale dell'oggetto restituito.

### Conto economico (Appendice B.2)

$$
\text{MOL} = \text{value added} - \text{costi per il personale}
$$

$$
\text{reddito operativo aziendale} = \text{reddito operativo} + \text{investing result}
$$

$$
\text{risultato prima delle imposte} = \text{reddito operativo aziendale} + \text{componenti non ricorrenti} + \text{financing result}
$$

con, per un esercizio in forma civilistica:

- valore della produzione = A1 + A2 e A3 + A4 + A5, parte caratteristica (al netto delle parti
  spostate dalle rettifiche `investing` e `non_recurring`); il totale A è usato quando fornito;
- external costs (costi esterni) = B6 + B7 + B8 + B11 + B14, ciascuna parte caratteristica, B8
  anche al netto dei canoni di leasing (con segno; decisione E1 del 2026-11-09); value added
  (valore aggiunto) = valore della produzione − external costs; `materials_consumed` = B6 + B11;
- margine operativo lordo (MOL, EBITDA) = value added − B9; ammortamenti e svalutazioni (B10),
  accantonamenti (B12, B13), investing result, financing result e componenti non ricorrenti sono
  sotto di esso. La definizione è restituita a parole in `ebitda_definition`, pronta per i
  gruppi `ratios.*`;
- ammortamenti = B10 a e b, + B10 c con la classificazione
  `write_downs_of_fixed_assets: operating` (la predefinita, P1), + ammortamento dei beni in
  leasing;
- other charges below EBITDA (altri oneri sotto il MOL) = B12 + B13, + B10 d con la
  classificazione `write_downs_of_receivables: operating` (la predefinita, E3), ciascuna parte
  caratteristica (E2);
- income from operating investments (proventi da partecipazioni, voce C15, di immobilizzazioni
  finanziarie operative) = C15 quando le immobilizzazioni finanziarie (B.III) sono interamente
  `operating` per l'esercizio, zero altrimenti (decisione P2 del 2026-11-09);
- reddito operativo della gestione caratteristica (margine operativo netto, MON) = MOL −
  ammortamenti − other charges below EBITDA + income from operating investments;
- investing result (risultato della gestione accessoria) = parti `investing` di A5 − parti
  `investing` di B14 + C15 (salvo che B.III sia operativa) + C16 + (D18 − D19), − B10 c e
  − B10 d solo con la classificazione `investing`; il totale D è usato quando fornito;
- reddito operativo aziendale (EBIT nella prassi italiana) = reddito operativo + investing
  result;
- componenti non ricorrenti = Σ proventi non ricorrenti − Σ oneri non ricorrenti, dalle
  rettifiche `non_recurring`: zero, con la convenzione "no item labelled non-recurring", se
  assenti; nullo, ed elencato in `not_computed`, quando l'esercizio non fornisce il conto
  economico (E6);
- financing result (risultato della gestione finanziaria) = C17-bis − C17 (C17 più gli
  interessi dei beni in leasing, restituito come `interest_and_other_financial_charges`) − la
  rivalutazione del TFR (interest cost) considerato debt-like, quando fornita (P2). C16,
  che comprende gli interessi sulle disponibilità liquide e sui crediti finanziari della
  posizione finanziaria netta, è nell'investing result (categoria investing di IFRS 18), quindi
  il financing result è al lordo di quegli interessi;
- utile netto = risultato prima delle imposte − voce 20, imposte come fornite.

I due subtotali corrispondono ai subtotali richiesti da IFRS 18, "operating profit or loss" e
"profit or loss before financing and income taxes", secondo le convenzioni di questa pagina (la
decisione P1 modifica R16): le voci indicate come non ricorrenti sono escluse da
`operating_profit`, mentre IFRS 18 le terrebbe nella categoria operativa, e le svalutazioni
seguono le loro classificazioni. IFRS 18 si applica a chi redige il bilancio secondo gli IFRS e
l'art. 2425 a chi applica gli OIC, e la riclassificazione riconduce entrambi agli stessi
subtotali. `operating_profit` esclude l'area degli investimenti (proventi da partecipazioni e da
liquidità, D18 e D19) e le componenti non ricorrenti.

**Svalutazioni e rivalutazioni** (Decision 17 di SPEC-0005, modificata da E3 e P1 del
2026-11-09). D18 e D19 (attività finanziarie) sono nell'investing result, sotto il reddito
operativo e sopra il reddito operativo aziendale; nessuna rettifica le sposta. B10 c
(svalutazioni delle immobilizzazioni materiali e immateriali) è operativa come predefinito, sotto
il MOL insieme agli ammortamenti (IFRS 18; OIC 12 paragrafo 75); la classificazione
`write_downs_of_fixed_assets: investing` la colloca nell'investing result. B10 d (svalutazioni
dei crediti dell'attivo circolante e delle disponibilità liquide) è operativa come predefinito,
negli altri oneri sotto il MOL, come fa la prassi IFRS 18 per la svalutazione dei crediti
commerciali; la classificazione `write_downs_of_receivables: investing` la colloca
nell'investing result. Il MOL è lo stesso con l'una o l'altra classificazione. Una svalutazione
non ricorrente si isola con una rettifica `non_recurring`. B10 a-c fornita come importo unico
(bilancio in forma abbreviata, art. 2435-bis) con la sua parte B10 a e b e senza B10 c dà B10 c
come differenza, e tutti i livelli sono calcolati (correzione D1-IS del 2026-11-09: più dati non
danno mai meno output); un passaggio mostra B10 a-c − B10 a e b con gli importi, una convenzione
indica la lettura e l'assegnazione indica il gruppo. Con B10 c operativa, il gruppo dà gli stessi output con o senza le sue
parti; con la classificazione `investing`, un gruppo fornito senza alcuna parte è letto come
ammortamenti, e le convenzioni lo indicano (Decision 8).

**Proventi e oneri seguono la voce patrimoniale collegata** (decisione P2 del 2026-11-09). I
proventi da partecipazioni (C15) seguono le immobilizzazioni finanziarie (B.III): quando B.III è
interamente `operating` per l'esercizio (classificazione o override), C15 è
`income_from_operating_investments`, sotto il MOL e nel reddito operativo; il MOL non cambia.
D18 e D19 restano nell'investing result, e così C16, la cui parte relativa a B.III non è fornita
separatamente. Con B.III non fornita, pari a zero, o senza stato patrimoniale per l'esercizio,
non vi è alcun investimento operativo da seguire: `income_from_operating_investments` è zero e
C15 resta nell'investing result, anche con la classificazione `operating`, e le convenzioni lo
indicano (D2-IS). Con B.III ripartita tra più destinazioni per l'esercizio, i proventi di
ciascuna parte non sono forniti: C15 resta nell'investing result e le convenzioni lo indicano.
Con B.III fornita e diversa da zero e la sua classificazione mancante, C15 la attende solo
quando C15 è fornita e diversa da zero (A3): income from operating investments, reddito
operativo, investing result e i livelli sottostanti non sono calcolati e sono elencati in
`not_computed` con il pointer della classificazione e le destinazioni ammesse (R8). C15 fornita
pari a zero non attende nulla; C15 non fornita manca con il proprio pointer.

La rivalutazione del TFR (interest cost) è compresa in B9. Quando una rettifica
`debt_like_provision` considera il TFR debt-like, il suo campo facoltativo `interest_cost` (un
valore per esercizio, zero o più, al massimo quanto resta di B9) sposta quell'importo dai costi
per il personale e dal MOL al financing result; è una riga di entrambe le riconciliazioni, e il
risultato prima delle imposte non cambia. Poiché `interest_and_other_financial_charges` resta
C17 più gli interessi dei beni in leasing, il financing result è allora C17-bis −
`interest_and_other_financial_charges` − la rivalutazione, e le convenzioni lo indicano. La
rivalutazione si sposta anche quando B9 non è fornita: il financing result è calcolato, i costi
per il personale e il MOL no, e la verifica `interest_cost_within_personnel_costs` è riportata
come non verificata (D3-IS). Senza di esso nulla si sposta e le convenzioni lo indicano. `interest_cost` è accettato solo con `aggregate` `employee_severance_indemnity` e solo
per un esercizio in cui la rettifica si applica (`FM-E001` altrimenti); oltre quanto resta di B9
è `FM-E026`.

**Riconciliazioni** (R19). `reconciliations.operating_profit` e
`reconciliations.profit_before_financing_and_taxes` partono da A − B del conto economico
civilistico, elencano ogni importo spostato (svalutazioni, parti `investing` e `non_recurring`,
righe del leasing, rivalutazioni del TFR, C15, C16 e D) con i pointer effettivamente letti
(`sources`: D18 e D19 entrambe quando sono lette entrambe; il gruppo B10 a-c, con la correzione,
quando B10 c è letta come assente; per una rettifica, l'importo dell'esercizio che sposta, per
esempio `/adjustments/0/amounts/0/payments` o `/adjustments/1/interest_cost/0`), e arrivano al subtotale, in modo esatto. Il risultato prima delle imposte è
uguale ad A − B ± C ± D e l'utile netto alla voce 21, salvo la differenza dei leasing, canoni −
ammortamento − interessi (E4).

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

Il conto economico della stessa impresa (forma ordinaria, art. 2425):

```python
from decimal import Decimal

import fairmark as fm

r = fm.statements.reorganise(
    [
        {
            "label": "2024",
            "statutory": {
                "income_statement": {
                    "revenue": "5000000",
                    "change_in_product_inventories": "40000",
                    "own_work_capitalised": "10000",
                    "other_revenue_and_income": "50000",
                    "purchases": "2200000",
                    "services": "1244000",
                    "use_of_third_party_assets": "150000",
                    "personnel_costs": "1000000",
                    "depreciation_and_write_downs": {
                        "depreciation_and_amortisation": "180000",
                        "write_downs_of_fixed_assets": "20000",
                        "write_downs_of_receivables": "15000",
                    },
                    "change_in_raw_material_inventories": "-30000",
                    "provisions_for_risks": "10000",
                    "other_provisions": "5000",
                    "other_operating_expenses": "40000",
                    "income_from_equity_investments": "12000",
                    "other_financial_income": "3000",
                    "interest_and_other_financial_charges": "60000",
                    "exchange_gains_and_losses": "-1000",
                    "revaluations_of_financial_assets": "2000",
                    "write_downs_of_financial_assets": "8000",
                    "income_taxes": "64000",
                    "profit_or_loss_for_the_year": "150000",
                }
            },
        }
    ]
)
inc = r.years[0]["income_statement"]
assert inc["ebitda"] == Decimal("496000")  # 5100000 − 3604000 − 1000000
assert inc["operating_profit"] == Decimal("266000")  # 496000 − (180000 + 20000) − (10000 + 5000 + 15000)
assert inc["investing_result"] == Decimal("9000")  # 12000 + 3000 + 2000 − 8000
assert inc["profit_before_financing_and_taxes"] == Decimal("275000")
assert inc["net_income"] == Decimal("150000")  # voce 21
assert inc["reconciliations"]["operating_profit"]["start"] == Decimal("266000")  # A − B
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

### Conto economico civilistico (Appendice A.2)

| Riferimento | Campo | Livello | Scelta |
| --- | --- | --- | --- |
| A1 | `revenue` | valore della produzione | fissa |
| A2, A3 | `change_in_product_inventories` (con segno, positiva per un aumento) | valore della produzione | fissa |
| A4 | `own_work_capitalised` | valore della produzione | fissa |
| A5 | `other_revenue_and_income` | valore della produzione, parte caratteristica | predefinita |
| B6, B11 | `purchases`, `change_in_raw_material_inventories` (con segno, positiva per una diminuzione) | value added (`materials_consumed`) | fissa |
| B7, B8 | `services`, `use_of_third_party_assets` | value added | fissa |
| B9 | `personnel_costs` | MOL | fissa |
| B10 a, b | `depreciation_and_write_downs.depreciation_and_amortisation` (oppure `value_adjustments_of_fixed_assets`, B10 a-c, al netto di B10 c) | ammortamenti | fissa |
| B10 c | `depreciation_and_write_downs.write_downs_of_fixed_assets` | ammortamenti, oppure investing result con la classificazione `investing` | predefinita |
| B10 d | `depreciation_and_write_downs.write_downs_of_receivables` | other charges below EBITDA, oppure investing result con la classificazione `investing` | predefinita |
| B12, B13 | `provisions_for_risks`, `other_provisions` | reddito operativo | fissa |
| B14 | `other_operating_expenses` | value added, parte caratteristica | predefinita |
| C15 | `income_from_equity_investments` | investing result; income from operating investments quando B.III è interamente `operating` (P2) | segue B.III |
| C16 | `other_financial_income` | investing result | fissa |
| C17, C17-bis | `interest_and_other_financial_charges`, `exchange_gains_and_losses` (con segno, positiva per un utile netto) | financing result | fissa |
| D18, D19 | `revaluations_of_financial_assets`, `write_downs_of_financial_assets` | investing result | fissa |
| 20, 21 | `income_taxes` (con segno), `profit_or_loss_for_the_year` (con segno) | imposte come fornite; verifica | fissa |

I totali `value_of_production` (A), `costs_of_production` (B), il `total` di B10,
`difference_between_value_and_costs_of_production` (A − B), `financial_income_and_charges` (C),
`value_adjustments_of_financial_assets` (D) e `profit_before_tax` sono facoltativi, con segno e
verificati. Il totale A sostituisce le sue voci nel valore della produzione, e il totale D le
voci D18 e D19 nell'investing result, quando le voci non sono fornite.

### Conto economico riclassificato (Appendice B.2)

`revenue`, `value_of_production`, `purchases`, `materials_consumed`, `value_added`,
`external_costs`, `personnel_costs`, `ebitda`, `depreciation_and_amortisation`,
`other_charges_below_ebitda`, `income_from_operating_investments`, `operating_profit`,
`investing_result`, `profit_before_financing_and_taxes`, `non_recurring_result`,
`financing_result`, `interest_and_other_financial_charges`, `profit_before_tax`,
`income_taxes` e `net_income`, tutti facoltativi. MOL (value added − costi per il personale),
reddito operativo aziendale (reddito operativo + investing result), risultato prima delle
imposte e utile netto sono derivati quando non forniti e verificati in modo esatto quando
forniti, e così il value added (valore della produzione − `external_costs`) e il reddito
operativo (MOL − ammortamenti − `other_charges_below_ebitda` +
`income_from_operating_investments`), con `external_costs` (con segno),
`other_charges_below_ebitda` e `income_from_operating_investments` (zero o più) definiti come
nella forma civilistica (E1, E2, P2); per ottenere il reddito operativo derivato, fornire
`income_from_operating_investments`, zero se non ve ne sono. Le
riconciliazioni con A − B richiedono la forma civilistica e sono elencate in `not_computed`
senza un pointer da fornire.

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
| `write_downs_of_receivables` | `operating` (B10 d negli other charges below EBITDA), `investing` (B10 d nell'investing result) | `operating`, elencata ogni volta che un esercizio civilistico fornisce il conto economico |
| `write_downs_of_fixed_assets` | `operating` (B10 c negli ammortamenti), `investing` (B10 c nell'investing result) | `operating`, elencata ogni volta che un esercizio civilistico fornisce il conto economico |

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
| `investing` | A5 o B14 | investing result | `aggregate`; `related_non_operating_asset` facoltativo (riportato) |
| `non_recurring` | ogni aggregato del conto economico dell'Appendice A.2 salvo D18 e D19 | componenti non ricorrenti | `aggregate` |
| `lease` | scrittura doppia (sotto) | operating fixed assets, PFN, patrimonio netto; B8, B10, C17 | `amounts`: `asset`, `liability`, `liability_current`, `prepayment`, `payments`, `depreciation`, `interest` |

Una rettifica `non_recurring` indica una parte di entità o incidenza eccezionale (art. 2427 n. 13,
OIC 12): esce dal suo aggregato e dai subtotali superiori ed entra nelle componenti non
ricorrenti. Una rettifica `investing` sposta proventi od oneri di un'attività non operativa (per
esempio l'affitto di un immobile non strumentale) da A5 o B14 all'investing result.

Una rettifica `lease` rappresenta un leasing con il metodo finanziario (OIC 12): il
bene entra negli operating fixed assets; il debito entra nei debiti finanziari lordi della
posizione finanziaria netta (decisione P3; ESMA32-382-1138 paragrafo 180), con la parte
`liability_current` corrente; il maxicanone
(`prepayment`) esce dalla voce D dell'attivo (la voce separata quando fornita, altrimenti la
parte esposta in C.II, che esce allora da C.II); bene − debito − maxicanone è l'effetto sul
patrimonio netto, aggiunto al patrimonio netto con la sua formula nell'explain. Nel conto
economico i canoni escono da B8, l'ammortamento si aggiunge agli ammortamenti e gli interessi a
C17. Non sono calcolate imposte differite, quindi il risultato prima delle imposte e l'utile
netto differiscono dagli importi civilistici per canoni − ammortamento − interessi: canoni e
ammortamento sono righe di entrambe le riconciliazioni con A − B, gli interessi sono nel
financing result. Ogni importo spostato è elencato in `adjustments_applied` con la sua
`line`.

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
| `income_statement` | I livelli dell'Appendice B.2 (si veda [Formula](#conto-economico-appendice-b2)), `ebitda_definition` e `reconciliations`; le voci che portano il nome di una voce civilistica (`revenue`, `purchases`, `materials_consumed`, `personnel_costs`, `depreciation_and_amortisation`, `interest_and_other_financial_charges`) sono importi della parte caratteristica, dopo le rettifiche |
| `assignments` | Ogni aggregato fornito: riferimento, pointer, importo, destinazione, origine (`mapping` o `analyst`) |
| `adjustments_applied` | Ogni rettifica: tipo, etichetta, importo, motivo, origine, destinazione; per un leasing, una voce per importo con la sua `line` |
| `checks` | Ogni relazione per identificativo: `reconciled` o `not_checked`, con i suoi pointer |
| `not_computed` | Per blocco, ogni output mancante con i pointer degli input mancanti e, per una classificazione mancante, le destinazioni ammesse; la voce ha la stessa forma dei metodi `ratios.*` (`output`, `missing`, `permitted`) |

| Output | Alimenta |
| --- | --- |
| `net_financial_position.position` | `net_financial_position` di `dcf.fcff` e di `ratios.capital_structure` |
| `non_operating_assets` | `non_operating_assets` di `dcf.fcff` |
| `debt_like_items` | `debt_like_items` di `dcf.fcff` |
| `current_non_current` | `ratios.liquidity` e `ratios.capital_structure` |
| `invested_capital` | `invested_capital` di `ratios.profitability` |
| `operating_profit` | `ebit` di `dcf.free_cash_flows`; `ebit` di `ratios.profitability` per il ROI sul capitale investito (le attività non operative entrano nel bridge al loro valore e sono escluse dal capitale investito) |
| `profit_before_financing_and_taxes` | `ebit` di `ratios.debt_service` (coperture degli oneri finanziari) e di `ratios.profitability` per il ROA sul totale attivo |
| `depreciation_and_amortisation` | `depreciation_and_amortisation` di `dcf.free_cash_flows` |
| `revenue`, `ebitda` con `ebitda_definition`, `net_income` | `ratios.profitability`; `ebitda` anche `ratios.debt_service` |
| `interest_and_other_financial_charges` | `interest` di `ratios.debt_service` |
| `purchases` | `purchases` di `ratios.working_capital`; `cost_of_sales` non è restituito (Decision 13): un conto economico per natura non lo contiene |

## Valori predefiniti

Ogni valore predefinito è elencato in `meta.defaults_applied` e in `explain.conventions`, in
quest'ordine:

1. per esercizio, ogni aggregato civilistico fornito la cui destinazione è una predefinita
   dell'Appendice A (B.I, B.II, C.II, C.III, C.IV, voce D dell'attivo, fondi per rischi e oneri B,
   trattamento di fine rapporto C, debiti D, voce E del passivo; poi A5 verso
   `value_of_production`, B14 verso `external_costs`, B10 c verso
   `depreciation_and_amortisation` e B10 d verso `other_charges_below_ebitda`, ciascuna finché la
   sua classificazione non è fornita): `field` è il pointer della voce,
   `value` la destinazione, e la convenzione riporta "assigned to". Un aggregato è fornito quando
   è fornito il suo importo: un nodo che ne contiene solo una parte (per esempio
   `receivables.trade`) non basta;
2. `treatments.write_downs_of_receivables` e `treatments.write_downs_of_fixed_assets`,
   `operating` se non fornite, ciascuna elencata ogni volta che un esercizio civilistico fornisce
   il conto economico, perché decidono che cosa è calcolato (decisione A4 del 2026-11-09, che
   modifica F1); quando una di queste classificazioni predefinite colloca una B10 c o una B10 d
   diversa da zero, una convenzione che inizia con "warning: " indica il valore predefinito e il
   subtotale che sposta (ADR 0014, test 4 di ammissione dei valori predefiniti);
3. ogni voce considerata pari a zero perché non fornita ([Calcolo sui dati
   disponibili](#calcolo-sui-dati-disponibili)): `value` è `"0"` e la convenzione riporta "not
   given, set to".

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
| Voce A non fornita | zero, lo stesso importo per patrimonio netto e totale attivo | predefinito, elencato (D1) |
| Parti "di cui" non fornite | assenti nel loro aggregato, anche quando l'aggregato non è fornito | predefinito, elencato (D2) |
| Derivati | passivi classificati come finanziari: non correnti nella posizione finanziaria netta; attivi secondo B.III (non correnti) o C.III (correnti) | fisso, indicato (D3) |
| A5, B14 | parte caratteristica; parti `investing` e non ricorrenti tramite rettifica | predefinito, elencato (Decision 7) |
| C15 | investing result; income from operating investments quando B.III è fornita, diversa da zero e interamente `operating` per l'esercizio; attende la classificazione di B.III quando C15 e B.III sono fornite e diverse da zero e la classificazione manca | segue B.III (P2, A3, D2-IS) |
| C16, D18, D19 | investing result | fisso |
| C17, C17-bis | financing result | fisso |
| MOL | value added − costi per il personale, restituito come `ebitda_definition` | fisso (Decision 8) |
| Svalutazioni e rivalutazioni | D18, D19 nell'investing result; B10 c e B10 d `operating` (sotto il MOL) oppure `investing` | `operating`, elencato (Decision 17, E3, P1) |
| B10 a-c come importo unico | con B10 c `investing`: ammortamenti salvo che sia fornita la sua parte B10 c | fisso, indicato (Decision 8) |
| Rivalutazione del TFR (interest cost) | nei costi per il personale (B9); con il TFR debt-like e il suo `interest_cost`, nel financing result | in B9, indicato quando il TFR è debt-like (P2) |
| Leasing | come esposti; metodo finanziario tramite rettifica, senza imposte differite | come esposti (Decision 9) |
| Imposte | voce 20 come fornita; nessun calcolo di IRES, IRAP, imposte differite o NOPAT | fisso |

## Verifiche

Ogni relazione è verificata in modo esatto quando entrambi i lati sono forniti, e `FM-E026`
riporta i due importi, la differenza, i pointer e la correzione quando differiscono. Un lato è
costruito solo con voci fornite dall'utente: la voce A o una parte "di cui" considerate pari a
zero non entrano mai in una verifica, e il messaggio e i `pointers` di una relazione verificata
indicano solo voci fornite. Una parte esposta all'interno di C.II o di D conta come fornita: la
sua voce separata è allora zero, perché la parte è già compresa in C o in D. Senza totale
attivo, per esempio, il totale attivo ricostruito come A + B + C + D è confrontato quando la
voce A è fornita e la voce D è fornita o esposta all'interno di C.II.

Forma civilistica: B.I e B.II nette = lordo − fondo; totale B, totale C, totale del patrimonio
netto, totale attivo e totale passivo e patrimonio netto rispetto alle loro voci; totale attivo
= totale passivo e patrimonio netto; parti esigibili oltre l'esercizio e parti "di cui" entro il
loro aggregato; utile di A.IX = voce 21; ripartizione di un override = aggregato dell'esercizio;
le scadenze di una ripartizione dei debiti verso soci per finanziamenti (si veda
[Classificazioni](#classificazioni)); i totali del conto economico (A, B, B10 a-c, B10, A − B,
C, D, risultato prima delle imposte, voce 21); B10 c, o in sua assenza B10 a e b, entro
B10 a-c; la parte corrente di una passività per leasing entro la passività; la rivalutazione del
TFR entro quanto resta di B9. Forma riclassificata: operating working capital, capitale investito, total
funds invested, posizione finanziaria netta (rispetto alla sua composizione e alle sue scadenze)
e total sources rispetto alle loro componenti, e total funds invested = total sources; MOL,
reddito operativo aziendale, risultato prima delle imposte e utile netto rispetto alla loro
derivazione secondo l'Appendice B.2, e value added e reddito operativo rispetto alla loro.

I livelli del conto economico civilistico sono costruiti con le stesse voci dei suoi totali,
quindi il risultato prima delle imposte è uguale ad A − B ± C ± D e l'utile netto alla voce 21,
salvo le rappresentazioni dei leasing, e ogni riconciliazione quadra in modo esatto; una
differenza è un difetto (`FM-E012`).

Nella forma civilistica total funds invested è uguale alle fonti ogni volta che tutte le voci
sono fornite, perché l'identità discende dalle relazioni precedenti. Quando differiscono, le
voci fornite non quadrano una volta contate le voci separate considerate pari a zero (la voce A,
e le voci D ed E esposte all'interno di C.II o di D): il risultato è `FM-E026` (approvazione del
maintainer del 2026-11-08), che indica solo quelle voci e la correzione: un importo fornito è
diverso, oppure una di queste voci considerate pari a zero non è zero. Una parte "di cui" considerata
assente sposta un importo all'interno di un solo lato e non è indicata. Con tutte le voci
fornite, una differenza è un difetto (`FM-E012`). Questa relazione non indica pointer in
`checks`: i suoi lati sono output calcolati, e il suo stato tiene conto delle voci considerate
pari a zero.

Per i bilanci depositati in unità di euro, fornire la riserva da arrotondamento nelle altre
riserve (A.VI), in modo che i totali quadrino esattamente.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Entrambe le forme in una voce; date non distanti un esercizio o dalla più recente; un importo negativo dove lo schema non lo prevede (compreso un importo di un leasing); un override di un altro aggregato; `beyond_one_year` su una parte `quasi_equity` di una ripartizione; una lista `amounts` di lunghezza diversa; un'etichetta, un motivo o un `related_non_operating_asset` vuoti; una rettifica su un esercizio riclassificato; una rettifica `non_recurring` di D18 o D19; una rettifica `investing` di un aggregato diverso da A5 o B14; un `interest_cost` su un fondo debt-like diverso dal TFR, di lunghezza diversa, o per un esercizio in cui la rettifica non si applica; un campo o un valore sconosciuti |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |
| `FM-E009` | Una data inesistente o fuori dall'intervallo 1900-2199 |
| `FM-E012` | Difetto: total funds invested diverso dalle fonti di un esercizio civilistico con tutte le voci fornite; una riconciliazione con A − B che non quadra; il risultato riclassificato diverso da quello civilistico per una causa diversa dai leasing |
| `FM-E026` | Una relazione del bilancio non quadra; total funds invested di un esercizio civilistico diverso dalle fonti una volta contate le voci separate considerate pari a zero (indicate nel messaggio); una rettifica, o la rivalutazione del TFR, supera quanto resta nella sua voce di origine; B10 c, o in sua assenza B10 a e b, supera il gruppo B10 a-c a cui appartiene; oppure la parte corrente di una passività per leasing supera la passività |

## Limiti e note di prassi

- **Imposte anticipate** (C.II 5-ter) restano nei crediti, e quindi nell'operating working
  capital. Per escluderle dal capitale circolante, spostarle con una rettifica
  `non_operating_asset` dei `receivables` e indicarne il motivo.
- **Proventi delle immobilizzazioni finanziarie** (decisione P2 del 2026-11-09): solo C15 segue
  B.III classificata `operating`. C16 comprende gli interessi sui crediti delle immobilizzazioni
  finanziarie (B.III.2), che lo schema non fornisce separatamente, e resta nell'investing result;
  con una ripartizione di B.III vi resta anche C15, perché i proventi di ciascuna parte non sono
  forniti.
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
  bridge, e il suo campo facoltativo `interest_cost` sposta la rivalutazione dell'esercizio da B9
  al financing result (P2).
- **Debiti verso imprese del gruppo** (D.8-D.11: verso imprese controllate, collegate,
  controllanti e sottoposte al controllo delle controllanti) restano nel resto dei debiti D,
  nell'operating working capital: questa versione non ne separa la parte finanziaria (decisione
  D6 del 2026-11-08; issue separata). Quando una loro parte è finanziamento, fornirla in
  `payables.financial_debt`, oppure spostarla con una rettifica `non_current_trade_payables`, e
  indicarne il motivo.
- **Differenze di cambio.** C17-bis è un importo netto: una rettifica `non_recurring` di
  C17-bis sposta una parte di un utile netto su cambi; una perdita su cambi non ricorrente non è
  rappresentabile in questa versione (decisione E7;
  [issue #251](https://github.com/salvo8519/fairmark/issues/251)).
- **Leasing** sono rappresentati senza imposte differite (Decision 9): l'effetto fiscale della
  rappresentazione resta nella voce 20 come esposta.

## Accuratezza

Solo addizioni e sottrazioni di decimali, esatte; nessuna moltiplicazione, divisione o
arrotondamento.

## Verificato con

- Golden case in `tests/golden/statements.reorganise/` (lo stesso bilancio in forma ordinaria e
  abbreviata con output identici, per lo stato patrimoniale e per entrambi i prospetti, una
  classificazione mancante, pochi totali soltanto, classificazioni e override su più esercizi,
  un caso per ogni tipo di rettifica, crediti verso soci per versamenti ancora dovuti, dividendi
  deliberati, la rappresentazione di un leasing, le svalutazioni con entrambe le
  classificazioni, un esercizio in perdita, gli ammortamenti raggruppati, date in anni
  bisestili e di fine mese, la forma riclassificata dello stesso bilancio, un caso `FM-E026` per
  relazione con il suo gemello non verificato, e gli errori di input), i cui valori attesi
  provengono da `scripts/reference_statements.py`, un riferimento decimale esatto a 50 cifre
  scritto dalle appendici di SPEC-0005 senza leggere il motore; la sua sezione sul conto
  economico è stata scritta prima di quella del motore.
- `python/tests/test_reference_statements.py`: il motore rispetto a quel riferimento su 400
  chiamate casuali con seme fisso nelle due forme, per ogni output, riconciliazione, verifica,
  valore predefinito e voce di `not_computed`, comprese le voci considerate pari a zero, le
  ripartizioni dei debiti verso soci per finanziamenti con scadenze, le posizioni finanziarie
  nette riclassificate fornite per parti, ogni tipo di rettifica e la scrittura doppia del
  leasing.
- Proprietà in Rust: total funds invested è uguale alle fonti; il reddito operativo aziendale è
  uguale a reddito operativo + investing result; il risultato prima delle imposte ad A − B ± C ±
  D; l'utile netto alla voce 21; le riconciliazioni quadrano; la classificazione `operating` non
  cambia né il MOL né il reddito operativo aziendale.

## Fonti

- Codice civile, art. 2423-ter, 2424, 2425, 2435-bis e 2435-ter (schemi e dati comparativi);
  art. 2427, comma 1, n. 13 (elementi di entità o incidenza eccezionali).
- IFRS 18 *Presentation and Disclosure in Financial Statements* (in vigore dal 2027): i
  subtotali "operating profit or loss" (paragrafo 69(a)) e "profit or loss before financing and
  income taxes"; partecipazioni valutate con il metodo del patrimonio netto nella categoria
  investing (paragrafi 53(a), 54(a)).
- Banca d'Italia, analisi della stabilità finanziaria delle imprese, ed EBA/GL/2020/06, allegato
  3: usano MOL ed EBITDA (l'EBA elenca l'EBITDA tra le metriche per la concessione del credito)
  ma non li definiscono; la definizione di EBITDA qui è la Decision 8 di SPEC-0005.
- OIC 12 (composizione e schemi del bilancio d'esercizio; leasing e metodo finanziario);
  D.Lgs. 139/2015 (azioni proprie dedotte dal patrimonio netto dal 2016).
- ESMA32-382-1138, orientamento 39, paragrafi da 175 a 186, e Consob, Richiamo di attenzione n.
  5/21 (indebitamento finanziario netto; leasing nei debiti finanziari, paragrafo 180); si veda
  "Rispetto alla tabella ESMA" per dove Fairmark se ne discosta.
- Koller, Goedhart, Wessels, *Valuation*, "Reorganizing the financial statements".
- SPEC-0005, le sue decisioni da 1 a 18, le decisioni del maintainer da D1 a D6 del
  2026-11-08 ed E1-E7, P1-P5 del 2026-11-09; SPEC-0007 (posizione finanziaria netta); SPEC-0001 Decision 21 (voci del bridge e
  `MeasurementBasis`).
