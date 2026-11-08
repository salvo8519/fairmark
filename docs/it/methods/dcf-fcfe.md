---
source: methods/dcf-fcfe.md
source_hash: c18c1a9ebc58
---
# DCF, flussi di cassa disponibili per gli azionisti (`dcf.fcfe`)

Valuta direttamente il capitale proprio di un'impresa attualizzando i flussi di cassa
disponibili per gli azionisti (FCFE) al costo del capitale proprio lungo da uno a tre stage
(fasi del periodo di previsione esplicita), aggiunge un valore terminale con approccio equity
side, poi aggiunge le attività non operative e sottrae i debt-like items e le partecipazioni di
minoranza. Il debito è già nei flussi, attraverso gli oneri finanziari e il net borrowing (nuovi
finanziamenti al netto dei rimborsi), quindi non si sottrae alcuna posizione finanziaria netta.
Ogni scelta che modifica il valore è un input esplicito, come in [`dcf.fcff`](dcf-fcff.md), che
usa lo stesso motore.

Versione del metodo: 1. Stato: anteprima.

## Formula

$$
FCFE_t = FCFF_t - \text{oneri finanziari}_t \, (1 - t) + \text{net borrowing}_t
$$

$$
V = \sum_{k=1}^{N} FCFE_k \, DF_k + TV_N \, DF_{TV}
\qquad
DF_k = \prod_{i=1}^{k} (1 + k_{e,i})^{-1}
$$

$$
Equity = V + \sum \text{attività non operative} - \sum \text{debt-like items} - \sum
\text{partecipazioni di minoranza}
$$

$k_{e,i}$ è il costo del capitale proprio del periodo i: un unico tasso per tutti i periodi e
per il valore terminale (`cost_of_equity`), oppure un tasso per periodo
(`cost_of_equity_per_period`) con il tasso della fase terminale (`terminal_cost_of_equity`).
I FCFE si attualizzano al costo del capitale proprio, non al WACC, perché il FCFE è la cassa
che resta agli azionisti dopo che i finanziatori sono stati pagati (CFA Institute, Free Cash
Flow Valuation; Damodaran, *Investment Valuation*, capitolo 14).

I flussi si ottengono esattamente come in [`dcf.fcff`](dcf-fcff.md): l'orizzonte esplicito N è
la somma dei periodi degli stage, e ogni stage è `given`, `constant_growth` o `linear_decline`,
con le stesse formule, la stessa collocazione temporale dei flussi (fine periodo o metà
periodo, con il momento di attualizzazione del valore terminale) e gli stessi termini nominali
e reali con l'inflazione attesa. Gli importi sono FCFE: forniti direttamente, generati dal base
flow $CF_0$ (il FCFE del periodo base) oppure costruiti dalle voci.

## FCFE dalle voci

Uno stage `given` può ricevere i flussi come voci in `lines`, un oggetto per periodo, con
`ebit`, `depreciation_and_amortisation`, `capital_expenditure`,
`change_in_net_working_capital`, `other_adjustments` facoltative e, nella forma a importi,
`operating_taxes`, più le voci finanziarie `interest_expense` (oneri finanziari) e
`net_borrowing` (positivo per nuovi finanziamenti) in ogni voce. Il FCFE di ogni periodo è
costruito come in [`dcf.free_cash_flows`](dcf-free-cash-flows.md):

- con `notional_tax_rate` e `negative_ebit`, le imposte sull'EBIT derivano dall'aliquota e lo
  scudo fiscale sugli oneri finanziari usa la stessa aliquota; in un periodo con EBIT negativo
  e trattamento `zero` non ci sono imposte né scudo fiscale, quindi gli oneri finanziari sono
  sottratti per intero;
- con `operating_taxes` in ogni voce, l'aliquota dello scudo fiscale è il campo obbligatorio
  `interest_tax_rate`.

Le voci senza le voci finanziarie sono rifiutate: l'approccio equity side le richiede in ogni
periodo. Con `cash_flow_terms` real, gli oneri finanziari devono essere in termini reali: la
parte degli interessi nominali che compensa l'inflazione è, in termini reali, un rimborso di
capitale e va nel net borrowing. È una nota di documentazione: il motore non la verifica. Ogni passo della costruzione è nell'explain, con il prefisso "Build-up:", e la tabella
dei periodi indica che il flusso è stato costruito dalle voci.

## Tecniche del valore terminale con approccio equity side

L'oggetto `terminal` indica esattamente una tecnica con l'identificativo del suo metodo, come
in [`dcf.fcff`](dcf-fcff.md#tecniche-del-valore-terminale), calcolata dal proprio metodo al
costo del capitale proprio della fase terminale. La tecnica deve seguire l'approccio equity
side:

| Tecnica | Con approccio equity side |
| --- | --- |
| [`tv.gordon`](tv-gordon.md), [`tv.h_model`](tv-h-model.md), [`tv.limited_horizon`](tv-limited-horizon.md) | Accettate: seguono l'approccio dei flussi che ricevono, qui i FCFE |
| [`tv.exit_multiple`](tv-exit-multiple.md) | `earnings` (P/E) o `book_value` (P/BV); `ebitda`, `ebit` e `revenue` sono multipli asset side e sono rifiutati con `FM-E019` |
| [`tv.normalised_income`](tv-normalised-income.md) | Un utile netto normalizzato con `basis` `equity`; `enterprise` è rifiutato con `FM-E019` |
| [`tv.liquidation`](tv-liquidation.md) | `basis` `equity`, con i debiti finanziari tra le passività; `enterprise` è rifiutato con `FM-E019` |
| [`tv.value_driver`](tv-value-driver.md) | Rifiutato con `FM-E019`: i suoi parametri, il NOPAT e il rendimento del nuovo capitale investito (RONIC), sono grandezze a livello d'impresa (asset side), quindi la formula del value driver non è prassi accettata con approccio equity side (SPEC-0001 Decision 23); usare `tv.gordon` con un flusso normalizzato in N + 1, oppure un multiplo di uscita sugli utili o sul valore contabile del capitale proprio |

Un valore terminale di Gordon sull'ultimo FCFE fatto crescere porta in perpetuo il net
borrowing di quel flusso. Nella fase di crescita stabile il net borrowing dovrebbe essere
quello che mantiene costante il rapporto di indebitamento dell'impresa, come nel FCFE in
crescita stabile di Damodaran, *Investment Valuation*, capitolo 14; quando l'ultimo anno
esplicito ha nuovi finanziamenti o rimborsi non ricorrenti, si indichi un FCFE normalizzato in
N + 1 (`first_flow` = `normalised_next_flow`). È una nota di documentazione: nessuna avvertenza
è riportata.

## Dal valore attuale dei FCFE all'equity value

Il valore attuale dei FCFE espliciti e del valore terminale è il valore del diritto degli
azionisti sui flussi operativi. Tre tipi di voci completano l'equity value:

| Voce | Input | Segno |
| --- | --- | --- |
| Attività non operative | `non_operating_assets`: `{"label", "value"}`, valori pari a zero o positivi | aggiunte |
| Debt-like items | `debt_like_items`: `{"label", "value", "measurement_basis", "basis_note"}`, valori pari a zero o positivi | sottratti |
| Partecipazioni di minoranza | `non_controlling_interests`: `{"label", "value", "measurement_basis", "basis_note"}`, valori pari a zero o positivi | sottratte |

Le voci e i loro criteri di valutazione funzionano come in
[`dcf.fcff`](dcf-fcff.md#dal-value-of-operations-allenterprise-value-e-allequity-value): ogni
importo è misurato alla data di valutazione e preso come fornito, ogni debt-like item e ogni
partecipazione di minoranza dichiara il proprio `measurement_basis`, e le liste non fornite
sono riportate come "none given". Il risultato non contiene né il value of operations né
l'enterprise value.

Come si tratta ogni voce con l'approccio equity side:

- **Debito e posizione finanziaria netta.** Il FCFE paga già gli oneri finanziari e registra
  i nuovi finanziamenti e i rimborsi, quindi il debito è nei flussi. Una
  `net_financial_position` è rifiutata con `FM-E019`: sottrarla conterebbe il debito due volte.
- **Disponibilità liquide.** La posizione finanziaria netta compensa anche le disponibilità
  liquide, quindi la liquidità non si sottrae né si aggiunge attraverso di essa. La liquidità
  non necessaria all'attività operativa, i cui interessi attivi non sono nel FCFE, è
  un'attività non operativa: va indicata in `non_operating_assets`. La liquidità i cui
  interessi attivi sono nel FCFE, o che serve all'attività operativa, è già valutata dai flussi
  e non si aggiunge.
- **Debt-like items.** Si sottraggono solo le obbligazioni i cui flussi non sono nel FCFE, per
  esempio un obbligo pensionistico non finanziato i cui contributi di ripianamento non sono nel
  piano. Un'obbligazione i cui pagamenti sono già nel FCFE non va sottratta di nuovo; il motore
  non controlla i doppi conteggi.
- **Partecipazioni di minoranza.** Si sottraggono quando i FCFE sono quelli del gruppo
  consolidato, che includono il 100% dei flussi delle controllate non interamente possedute.

Un equity value negativo è riportato come calcolato, con un'avvertenza, e mai riportato a zero.

## Esempio

Un piano con tre anni di FCFE forniti, poi tre anni in cui la crescita si riduce linearmente
dal 7% al 2,5%, poi una crescita stabile del 2,5%, a un costo del capitale proprio dell'11,5%:

```python
import fairmark as fm

r = fm.dcf.fcfe(
    [
        fm.dcf.given([180, 195, 210]),
        fm.dcf.linear_decline(3, from_growth="0.07", to_growth="0.025"),
    ],
    cash_flow_terms="nominal",
    discount_rate_terms="nominal",
    cost_of_equity="0.115",
    timing="end_of_period",
    terminal=fm.dcf.gordon("0.025", first_flow="last_flow_grown"),
    non_operating_assets=[{"label": "Excess cash, interest income not in the FCFE", "value": 140}],
    non_controlling_interests=[
        fm.dcf.bridge_claim("20% of Subsidiary A", 85, measurement_basis="fair_value")
    ],
)
print([row["growth"] for row in r.periods][3:])  # 0.055, 0.040, 0.025
print(r.bridge["pv_explicit_and_terminal"])      # 2269.51242269056513224936544...
print(r.equity_value)                            # 2269.51... + 140 − 85 = 2324.51...
print(r.terminal_value_weight)                   # 0.61677753424248815718841...
assert r.equity_value - r.bridge["pv_explicit_and_terminal"] == 140 - 85
```

FCFE costruiti dalle voci con oneri finanziari e net borrowing, e un multiplo di uscita P/E:

```python
r = fm.dcf.fcfe(
    [
        fm.dcf.given_lines(
            [
                {"ebit": 400, "depreciation_and_amortisation": 60, "capital_expenditure": 80,
                 "change_in_net_working_capital": 10, "interest_expense": 30,
                 "net_borrowing": 20},
                {"ebit": 430, "depreciation_and_amortisation": 64, "capital_expenditure": 85,
                 "change_in_net_working_capital": 11, "interest_expense": 31,
                 "net_borrowing": 15},
            ]
        )
    ],
    notional_tax_rate="0.24",
    negative_ebit="zero",
    cash_flow_terms="nominal",
    discount_rate_terms="nominal",
    cost_of_equity="0.11",
    timing="end_of_period",
    terminal=fm.dcf.exit_multiple("12", "earnings", 300),
)
print([row["flow"] for row in r.periods])  # 274 − 30 × 0.76 + 20 = 271.20; 286.24
print(r.terminal["terminal_value"])        # 12 × 300 = 3600
```

Senza debito, i FCFE coincidono con i FCFF e `dcf.fcfe` a un costo del capitale proprio pari
al WACC dà l'equity value di `dcf.fcff` senza posizione finanziaria netta, fino all'ultima
cifra.

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `stages` | Da uno a tre stage di FCFE, in ordine: `{"periods", "projection", ...}` con `flows` o `lines` (`given`), `growth` (`constant_growth`) oppure `from_growth` e `to_growth` (`linear_decline`) | obbligatorio |
| `base_flow` | FCFE del periodo base $CF_0$, quando il primo stage è generato | obbligatorio per un primo stage generato; rifiutato altrimenti |
| `notional_tax_rate` | Aliquota fiscale sull'EBIT per gli stage con `lines`, usata anche per lo scudo fiscale sugli oneri finanziari; in alternativa a `operating_taxes` in ogni voce | nessuno; rifiutata senza `lines` |
| `negative_ebit` | `credit` o `zero`, per gli stage con `lines` e `notional_tax_rate` | obbligatorio con `notional_tax_rate`; rifiutato altrimenti |
| `interest_tax_rate` | Aliquota dello scudo fiscale per gli stage con `lines` e `operating_taxes` | obbligatoria in quel caso; rifiutata altrimenti |
| `cash_flow_terms` | `nominal` oppure `real`: termini di ogni importo e di ogni tasso di crescita | obbligatorio |
| `discount_rate_terms` | `nominal` oppure `real`: termini dei costi del capitale proprio; se diversi da `cash_flow_terms`, l'inflazione attesa è obbligatoria e i tassi sono convertiti nei termini dei flussi di cassa | obbligatorio |
| `inflation`, `inflation_per_period`, `terminal_inflation` | Inflazione attesa, come in [`dcf.fcff`](dcf-fcff.md#termini-nominali-e-reali) | nessuna |
| `cost_of_equity` | Un unico costo del capitale proprio per tutti i periodi e per il valore terminale, frazione decimale | questo oppure `cost_of_equity_per_period` |
| `cost_of_equity_per_period` | Un costo del capitale proprio per ogni periodo dell'orizzonte esplicito | questo oppure `cost_of_equity` |
| `terminal_cost_of_equity` | Costo del capitale proprio della fase terminale | obbligatorio con `cost_of_equity_per_period`; rifiutato con `cost_of_equity` |
| `timing` | `end_of_period` oppure `mid_period` | obbligatorio |
| `terminal_discount_time` | `horizon` (t = N) oppure `mid_period` (t = N − 0.5) | obbligatorio con `mid_period`; rifiutato con `end_of_period` |
| `terminal` | Una tecnica con approccio equity side, indicata con l'identificativo del suo metodo | obbligatorio |
| `net_financial_position` | Rifiutata con `FM-E019`: il debito è nei FCFE | non accettata |
| `non_operating_assets` | Lista di `{"label", "value"}`, valori pari a zero o positivi, aggiunte | nessuna, riportato |
| `debt_like_items` | Lista di `{"label", "value", "measurement_basis", "basis_note"}`, valori pari a zero o positivi, sottratti | nessuno, riportato |
| `non_controlling_interests` | Partecipazioni di minoranza: lista di `{"label", "value", "measurement_basis", "basis_note"}`, valori pari a zero o positivi, sottratte | nessuna, riportato |

## Output

| Campo | Significato |
| --- | --- |
| `basis` | `equity` |
| `equity_value` | Valore attuale dei FCFE espliciti e del valore terminale, più le attività non operative, meno i debt-like items e le partecipazioni di minoranza |
| `pv_explicit_period` | Valore attuale dei FCFE espliciti |
| `terminal_value_weight` | Valore attuale del valore terminale diviso per la somma del valore attuale dei FCFE espliciti e del valore attuale del valore terminale; `null` quando la somma è zero |
| `periods` | Per periodo, come in `dcf.fcff`: stage, proiezione, crescita applicata, flusso, tasso di attualizzazione fornito, tasso usato, inflazione usata, fattore di sconto, valore attuale |
| `terms` | Termini dei flussi e dei tassi, conversione, tasso terminale fornito e usato, crescita perpetua in entrambi i termini |
| `terminal` | Tecnica e versione del suo metodo, tasso terminale, valore in N, momento di attualizzazione, fattore, valore attuale, output proprio della tecnica |
| `bridge` | `pv_explicit_and_terminal`, ogni attività non operativa, debt-like item e partecipazione di minoranza con la sua descrizione e il suo criterio di valutazione, il totale di ciascuna lista ed `equity_value` |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Approccio | equity side: FCFE attualizzati al costo del capitale proprio; equity value direttamente, senza posizione finanziaria netta | fissato dal metodo |
| FCFE di uno stage `given` | importi (`flows`) o voci con oneri finanziari e net borrowing (`lines`) | uno dei due, obbligatorio |
| Scudo fiscale sugli oneri finanziari | oneri finanziari × (1 − t), con t l'aliquota sull'EBIT o `interest_tax_rate`; nessuno in un periodo in cui si applica il trattamento `zero` | fisso |
| Stage, tasso di attualizzazione, termini, inflazione, collocazione temporale dei flussi, momento di attualizzazione del valore terminale | come in [`dcf.fcff`](dcf-fcff.md), con il costo del capitale proprio come tasso di attualizzazione | obbligatori |
| Tecnica del valore terminale | con approccio equity side, al costo del capitale proprio della fase terminale | obbligatoria |
| Voci del passaggio all'equity value | misurate alla data di valutazione come fornite; nessuna voce significa nessuna | nessuna |
| Segni delle voci | attività non operative aggiunte; debt-like items e partecipazioni di minoranza sottratti | fissi |
| Criterio di valutazione dei debt-like items e delle partecipazioni di minoranza | `book_value`, `market_value`, `fair_value`, `present_value`, `other` (con `basis_note`); riportato, non usato nel calcolo | obbligatorio per ogni voce |

Le avvertenze di `dcf.fcff` si applicano invariate (crescita su un flusso negativo, crescita
discontinua, momento di attualizzazione del valore terminale con la convenzione di metà
periodo, crescita perpetua inferiore all'inflazione), e un equity value negativo è riportato
come avvertenza, mai riportato a zero.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Come in [`dcf.fcff`](dcf-fcff.md#errori), con i campi del costo del capitale proprio al posto di quelli del WACC (per esempio `/terminal_cost_of_equity` insieme a `cost_of_equity`); un campo di `dcf.fcff` che questo metodo non ha, come `wacc`; `lines` senza `interest_expense` e `net_borrowing`; `interest_tax_rate` senza `lines` |
| `FM-E003` | Un costo del capitale proprio, fornito o convertito, oppure un'inflazione attesa è pari o inferiore a −100% (per esempio `/cost_of_equity_per_period/2`) |
| `FM-E004` | La crescita perpetua è pari o superiore al costo del capitale proprio della fase terminale nei termini dei flussi di cassa |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |
| `FM-E018` | Un parametro della tecnica del valore terminale è fuori dal suo dominio, come in `dcf.fcff` |
| `FM-E019` | È fornita una `net_financial_position`; oppure la tecnica del valore terminale segue l'approccio asset side: un multiplo di uscita su `ebitda`, `ebit` o `revenue`, un reddito normalizzato o una liquidazione con `basis` enterprise, oppure `tv.value_driver`, il cui NOPAT e il cui RONIC sono grandezze a livello d'impresa (Decision 23) |
| `FM-E024` | `cash_flow_terms` e `discount_rate_terms` sono diversi e non è fornita l'inflazione attesa |
| `FM-E025` | Un conteggio non corrisponde agli stage: flussi o voci, costi del capitale proprio o inflazione per periodo, oppure un base flow fuori posto |

Gli errori di validazione indicano il campo con il suo JSON Pointer e la correzione da
apportare; gli errori rilevati durante la lettura dell'input (un campo mancante o sconosciuto,
un tipo errato) indicano invece la riga e la colonna dell'input.

## Precisione

Il motore è quello di `dcf.fcff`: fattori di sconto con divisioni esatte (fine periodo) o con
una radice quadrata decimale (metà periodo), valori attuali per divisione per il fattore di
capitalizzazione. I casi di riferimento derivati dal calcolo indipendente sono verificati entro
$10^{-20}$.

## Verificato con

- Un'implementazione indipendente a 50 cifre, `scripts/reference_dcf.py` (`dcf_fcfe`), per ogni
  caso di riferimento derivato dal calcolo indipendente in `tests/golden/dcf.fcfe/`: tre stage
  con un costo del capitale proprio per periodo e le voci del passaggio all'equity value, un
  multiplo di uscita P/E, un utile netto normalizzato, flussi reali con inflazione con la
  convenzione di metà periodo, un equity value negativo; e su una griglia di stage, tassi e
  collocazioni temporali dei flussi.
- Stage con `lines`: la costruzione a 50 cifre di `scripts/reference_free_cash_flows.py`
  (`equity_stages_with_flows`) composta con `scripts/reference_dcf.py`, in entrambe le forme
  fiscali e con il trattamento `zero` di un EBIT negativo (`stage-with-build-up-lines-*.yaml`).
- Coerenza con `dcf.fcff` (test parametrizzati con seme fisso): senza debito e senza net borrowing, l'equity
  value di `dcf.fcfe` è uguale all'equity value di `dcf.fcff` sugli stessi flussi, con un WACC
  pari al costo del capitale proprio e le stesse voci, fino all'ultima cifra; anche su voci con
  oneri finanziari e net borrowing pari a zero.
- L'identità di R24 su voci casuali: equity value = valore attuale dei FCFE e del valore
  terminale + attività non operative − debt-like items − partecipazioni di minoranza.
