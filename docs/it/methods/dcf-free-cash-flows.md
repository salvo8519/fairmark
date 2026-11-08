---
source: methods/dcf-free-cash-flows.md
source_hash: c8b6e0873be5
---
# Costruzione dei flussi di cassa disponibili (`dcf.free_cash_flows`)

Costruisce il flusso di cassa operativo disponibile (FCFF) di ogni periodo di un business plan
dalle sue voci riclassificate: EBIT, operating taxes (imposte sull'EBIT), ammortamenti,
investimenti in immobilizzazioni (capex), variazione del capitale circolante netto e rettifiche
con etichetta. Se sono forniti anche gli oneri finanziari e il net borrowing (nuovi
finanziamenti al netto dei rimborsi), costruisce il flusso di cassa disponibile per gli
azionisti (FCFE). I flussi sono importi per periodo, non attualizzati: attualizzarli è compito
del DCF. La riclassificazione dei bilanci civilistici in queste voci è un passaggio distinto.

Versione del metodo: 1. Stato: anteprima.

## Formula

Per ogni periodo $t$:

$$
FCFF_t = EBIT_t - T_t + DA_t - CAPEX_t - \Delta NWC_t + \sum_k ADJ_{k,t}
$$

$$
FCFE_t = FCFF_t - INT_t \times (1 - \tau) + NB_t
$$

Le imposte sull'EBIT $T_t$ si forniscono in una sola di due forme:

- **aliquota sull'EBIT**: $T_t = \tau \times EBIT_t$, con l'aliquota fiscale $\tau$ fornita una
  volta per tutti i periodi. Quando $EBIT_t < 0$ decide il trattamento obbligatorio: `credit`
  mantiene $T_t = \tau \times EBIT_t$ (un risparmio d'imposta), `zero` pone $T_t = 0$. Il tax
  loss carry-forward (riporto delle perdite fiscali) non è modellato;
- **importi**: $T_t$ fornito per ogni periodo.

L'aliquota $\tau$ dello scudo fiscale sugli oneri finanziari nel FCFE è l'aliquota sull'EBIT
nella prima forma, e un input separato obbligatorio, `interest_tax_rate`, nella seconda. In un
periodo in cui si applica il trattamento `zero` (EBIT negativo, imposte da un'aliquota) lo
scudo fiscale sugli oneri finanziari è nullo: $\tau = 0$ e gli oneri finanziari sono sottratti
per intero. Il FCFE coincide allora con quello calcolato dall'utile netto (utile netto +
ammortamenti − capex − variazione del capitale circolante netto + rettifiche + net borrowing,
senza imposte su un risultato ante imposte negativo). Con `credit`, e in ogni periodo con EBIT
pari a zero o positivo, lo scudo si applica anche quando l'EBIT è inferiore agli oneri
finanziari.

## Esempio

L'illustrazione Boeing di Damodaran, dall'EBIT (1 − aliquota) pubblicato di 1.138 (milioni di
dollari), e un piano di due periodi con FCFE:

```python
from decimal import Decimal

import fairmark as fm

boeing = fm.dcf.free_cash_flows(
    [
        {
            "ebit": 1753,
            "operating_taxes": 615,  # 1.753 − 1.138, come pubblicato
            "depreciation_and_amortisation": 1517,
            "capital_expenditure": 1584,
            "change_in_net_working_capital": -105,  # una diminuzione libera cassa
        }
    ]
)
assert boeing.free_cash_flows_to_firm == [Decimal("1176")]

plan = fm.dcf.free_cash_flows(
    [
        {
            "ebit": "1200",
            "depreciation_and_amortisation": "350",
            "capital_expenditure": "420",
            "change_in_net_working_capital": "65",
            "interest_expense": "80",
            "net_borrowing": "-150",
        },
        {
            "ebit": "-250",
            "depreciation_and_amortisation": "90",
            "capital_expenditure": "60",
            "change_in_net_working_capital": "-35",
            "other_adjustments": [{"label": "Cessione di un magazzino", "value": "60"}],
            "interest_expense": "30",
            "net_borrowing": "100",
        },
    ],
    notional_tax_rate="0.24",
    negative_ebit="zero",
)
assert plan.free_cash_flows_to_firm == [Decimal("777.00"), Decimal("-125")]
assert plan.free_cash_flows_to_equity == [Decimal("566.20"), Decimal("-55")]
print(plan.conventions[-1])  # no interest tax shield in period 2 (negative EBIT, treatment zero)
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `periods` | Voci dei periodi 1..n, in ordine; almeno uno | obbligatorio |
| `periods[].ebit` | EBIT del periodo, in unità di valuta, di qualunque segno | obbligatorio |
| `periods[].operating_taxes` | Imposte sull'EBIT come importo, positivo per un'imposta pagata (forma a importi) | nessuno |
| `periods[].depreciation_and_amortisation` | Ammortamenti, zero o più, sommati | obbligatorio |
| `periods[].capital_expenditure` | Investimenti in immobilizzazioni (capex), zero o più, sottratti | obbligatorio |
| `periods[].change_in_net_working_capital` | Variazione del capitale circolante netto: positiva per un aumento, che assorbe cassa | obbligatorio |
| `periods[].other_adjustments` | Importi con etichetta `{label, value}`, positivi per un'entrata | nessuno |
| `periods[].interest_expense` | Oneri finanziari, zero o più; con `net_borrowing`, per tutti i periodi o per nessuno | nessuno |
| `periods[].net_borrowing` | Net borrowing: positivo per nuovi finanziamenti, negativo per rimborsi netti | nessuno |
| `notional_tax_rate` | Aliquota fiscale sull'EBIT, frazione decimale da 0 a meno di 1 (forma ad aliquota) | nessuno |
| `negative_ebit` | `credit` o `zero`; obbligatorio con `notional_tax_rate`, non accettato con `operating_taxes` | nessuno: va scelto |
| `interest_tax_rate` | Aliquota dello scudo fiscale sugli oneri finanziari, frazione decimale da 0 a meno di 1; obbligatoria con `operating_taxes` e le voci finanziarie, non accettata negli altri casi | nessuno |

Si fornisce `notional_tax_rate` oppure `operating_taxes` in ogni periodo, mai entrambi.

## Output

| Campo | Significato |
| --- | --- |
| `periods` | Una riga per periodo: ogni voce come fornita, `operating_taxes` usate, `ebit_after_taxes`, `other_adjustments_total`, `free_cash_flow_to_firm` e, con le voci finanziarie, `interest_tax_rate` applicata (0 dove si applica il trattamento `zero`), `interest_after_taxes` e `free_cash_flow_to_equity` (altrimenti `null`) |
| `free_cash_flows_to_firm` | FCFF dei periodi 1..n |
| `free_cash_flows_to_equity` | FCFE dei periodi 1..n; `null` senza le voci finanziarie |
| `tax_form` | `rate_on_ebit` o `amounts` |
| `negative_ebit` | Trattamento usato nella forma ad aliquota; `null` nella forma a importi |
| `interest_tax_rate` | Aliquota dello scudo fiscale usata; `null` senza le voci finanziarie |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Imposte sull'EBIT | aliquota sull'EBIT, o importi per periodo | nessuno: va fornita esattamente una forma |
| EBIT negativo (forma ad aliquota) | `credit`, `zero` | nessuno: va scelto |
| Tax loss carry-forward | non modellato | fisso |
| Aliquota dello scudo fiscale sugli oneri finanziari | l'aliquota sull'EBIT (forma ad aliquota), o `interest_tax_rate` (forma a importi); 0 in un periodo con EBIT negativo e trattamento `zero` | fissata dalla forma delle imposte e dal trattamento |
| Segno delle imposte sull'EBIT (forma a importi) | positivo per un'imposta pagata, sottratto; negativo per un credito d'imposta | fisso |
| Segno della variazione del capitale circolante netto | positivo per un aumento, sottratto | fisso |
| Segno del net borrowing | positivo per nuovi finanziamenti, sommato | fisso |
| Segno delle rettifiche | positivo per un'entrata, sommato | fisso |
| Importi | unità di valuta per periodo, non attualizzati | fisso |

Le dismissioni di immobilizzazioni e le altre entrate vanno in `other_adjustments` con segno
positivo: investimenti e ammortamenti non sono mai negativi.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Ammortamenti, investimenti in immobilizzazioni o oneri finanziari negativi (il messaggio indica il campo con il suo JSON pointer e il periodo); entrambe o nessuna forma delle imposte; importi delle imposte mancanti in un periodo; `negative_ebit` mancante con un'aliquota o fornito con gli importi; `interest_tax_rate` mancante o non accettata; oneri finanziari senza net borrowing, o voci finanziarie solo in alcuni periodi; un'aliquota fuori dall'intervallo da 0 a meno di 1; nessun periodo; un'etichetta di rettifica vuota; un campo mancante o sconosciuto |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Somme, sottrazioni e una moltiplicazione per ogni imposta: esatte fino alle 28 cifre
significative del tipo decimale. I casi di riferimento (golden case) sono confrontati
esattamente, o alla 28ª cifra significativa quando un prodotto richiede più cifre, con un
riferimento a 50 cifre. Il caso Boeing dell'illustrazione di Damodaran è confrontato alla
precisione pubblicata, in unità (tolleranza 0,5).

## Verificato con

- `tests/golden/dcf.free_cash_flows/damodaran-boeing-illustration-15-7.yaml`: Damodaran,
  *Investment Valuation*, 2ª edizione, illustrazione 15.7 (Boeing), FCFF 1.176 alla precisione
  pubblicata. L'illustrazione stampa l'EBIT (1 − aliquota) come 1.138, mentre
  1.753 × (1 − 0,35) = 1.139,45; il caso parte dal valore stampato di 1.138, e
  `boeing-rate-form-recomputed.yaml` registra l'FCFF di 1.177,45 che dà la forma ad aliquota.
- Gli altri casi di riferimento e un confronto con seme fisso su piani casuali provengono da un
  calcolo indipendente a 50 cifre (`scripts/reference_free_cash_flows.py`).
