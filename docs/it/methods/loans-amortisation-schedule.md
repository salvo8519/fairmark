---
source: methods/loans-amortisation-schedule.md
source_hash: 5bc4258b419e
---
# Piano di ammortamento (`loans.amortisation_schedule`)

Restituisce il piano di ammortamento di un finanziamento: per ogni periodo la rata, la quota
interessi, la quota capitale e il debito residuo, con i totali e la rettifica dell'ultima rata
che porta il debito residuo esattamente a zero. Sono disponibili tre tipi di ammortamento, con
un preammortamento iniziale facoltativo e un balloon facoltativo. Si usa per inserire gli
interessi di ogni anno e il debito residuo in un business plan e nella posizione finanziaria
netta, o per riconciliare un piano con quello della banca.

## Tipi di ammortamento

I nomi sono quelli del regolamento europeo sulle segnalazioni del credito (Regolamento (UE)
2016/867, Allegato IV, "tipo di ammortamento"):

| Tipo | Che cosa resta costante | Prassi italiana |
| --- | --- | --- |
| `french` | la rata: il totale di capitale e interessi è uguale in ogni rata | ammortamento alla francese (a rata costante) |
| `fixed_amortisation_schedule` | la quota capitale: il capitale rimborsato in ogni rata è uguale | ammortamento all'italiana (a quota capitale costante) |
| `bullet` | solo gli interessi: l'intero capitale si rimborsa con l'ultima rata | bullet (rimborso integrale del capitale con l'ultima rata) |

## Formula

Con il tasso periodico $i$, $n$ rate (l'intera durata), $g$ periodi iniziali di soli interessi
(preammortamento) e un balloon $B$ rimborsato con l'ultima rata, ogni periodo $k$ parte dal
debito residuo iniziale $D_{k-1}$ ($D_0 = P$):

$$
I_k = i\,D_{k-1}
\qquad
C_k =
\begin{cases}
0 & k \le g \text{ (preammortamento) o bullet prima dell'ultimo periodo} \\
A - I_k & \text{french} \\
(P - B) / (n - g) & \text{fixed amortisation schedule}
\end{cases}
\qquad
D_k = D_{k-1} - C_k
$$

La rata del periodo $k$ è $I_k + C_k$. Per `french`, $A$ è la rata costante di
[`loans.instalment`](loans-instalment.md) sugli $n - g$ periodi successivi al preammortamento,
con il balloon (la stessa implementazione):

$$
A = \frac{\left(P - B\,(1 + i)^{-(n-g)}\right) i}{1 - (1 + i)^{-(n-g)}}
$$

**Ultimo periodo.** La quota capitale del periodo $n$ è il debito residuo iniziale $D_{n-1}$,
balloon compreso, quindi il debito residuo finale è esattamente zero. La differenza tra
l'ultima rata, al netto del balloon, e la rata ordinaria di quel periodo è riportata come
`final_adjustment` (rettifica dell'ultima rata). Senza arrotondamento è zero salvo le ultime
cifre dell'aritmetica; con l'arrotondamento assorbe l'arrotondamento accumulato.

**Arrotondamento.** Quando è scelto, l'arrotondamento si applica all'importo che il tipo
mantiene costante, così che il tipo conservi il suo significato: la rata di un piano alla
francese, la quota capitale di un piano all'italiana. Le quote interessi non sono mai
arrotondate. Un piano bullet non ha importi costanti da arrotondare oltre ai propri interessi,
quindi l'arrotondamento non lo modifica.

La rata $k$ si paga alla fine del periodo $k$ (tempo $k$): questa versione del metodo supporta
solo rate posticipate.

## Esempio

L'esempio svolto di un finanziamento alle imprese in Watts, *Business and Financial
Mathematics*, sezione 6.1: 50.000 al 5% con capitalizzazione trimestrale, rimborsati con sei
rate semestrali arrotondate al centesimo. Il tasso periodico è $1.0125^2 - 1 = 0.02515625$,
calcolato con le funzioni in serie della conversione dei tassi, quindi esatto a circa
$10^{-27}$:

```python
from decimal import Decimal

import fairmark as fm

loan = fm.loans.amortisation_schedule(
    50000, 6, rate="0.05", compounding="quarterly", payment_frequency="semi_annual",
    amortisation_type="french", timing="ordinary", decimal_places=2, rounding_mode="half_up",
)
assert round(loan.periodic_rate, 20) == Decimal("0.02515625")  # series functions: 1e-27
first, last = loan.rows[0], loan.rows[-1]
assert first["instalment"] == Decimal("9082.24")
assert round(first["interest_portion"], 20) == Decimal("1257.8125")  # not rounded
assert round(last["instalment"], 2) == Decimal("9082.26")  # the book's final payment
assert last["outstanding_balance"] == 0
assert round(loan.final_adjustment, 4) == Decimal("0.0171")
assert round(loan.totals["interest"], 2) == Decimal("4493.46")

# Ammortamento all'italiana: 120.000 in 8 trimestri al 6% con capitalizzazione trimestrale.
fixed = fm.loans.amortisation_schedule(
    120000, 8, rate="0.06", compounding="quarterly", payment_frequency="quarterly",
    amortisation_type="fixed_amortisation_schedule", timing="ordinary",
)
assert [row["principal_portion"] for row in fixed.rows] == [Decimal(15000)] * 8
assert fixed.totals["interest"] == Decimal(8100)

# Due mesi di preammortamento, poi un piano alla francese con un balloon del 20% del capitale.
grace = fm.loans.amortisation_schedule(
    100000, 60, rate="0.048", compounding="monthly", payment_frequency="monthly",
    amortisation_type="french", timing="ordinary", principal_grace_periods=2,
    balloon_amount=20000,
)
assert grace.rows[0]["instalment"] == Decimal(400)
assert grace.rows[-1]["balloon_amount"] == Decimal(20000)
assert abs(grace.final_adjustment) < Decimal("1e-18")
```

## Input

| Campo | Significato | Default |
| --- | --- | --- |
| `principal` | Capitale finanziato, in unità di valuta, maggiore di 0 | obbligatorio |
| `rate`, `compounding`, `payment_frequency` | Tasso annuo nominale con la sua capitalizzazione e il numero di rate all'anno (forma a) | una forma di tasso obbligatoria |
| `periodic_rate` | Tasso per periodo di pagamento, frazione decimale, maggiore di −1 (forma b) | una forma di tasso obbligatoria |
| `number_of_instalments` | Intera durata, preammortamento compreso, da 1 a 10000 | obbligatorio |
| `timing` | `ordinary` (rata posticipata); `due` non è accettato dalla versione 1 | obbligatorio |
| `amortisation_type` | `french`, `fixed_amortisation_schedule` o `bullet` | obbligatorio |
| `principal_grace_periods` | Periodi iniziali di soli interessi, inferiori a `number_of_instalments`; non con `bullet` | `0` |
| `balloon_amount` | Capitale rimborsato con l'ultima rata, $0 \le B < P$; non con `bullet` | `0` |
| `rounding` | `decimal_places` (da 0 a 6) e `mode` (`half_up` o `half_even`), entrambi obbligatori | nessuno |

Le forme del tasso sono descritte in [`loans.instalment`](loans-instalment.md).

## Output

| Campo | Significato |
| --- | --- |
| `periodic_rate` | Tasso per periodo di pagamento utilizzato |
| `rows` | Una riga per periodo: `period`, `time`, `instalment`, `interest_portion`, `principal_portion`, `outstanding_balance` e, nell'ultima riga, `balloon_amount` |
| `totals` | `instalments`, `interest` e `principal` (le quote capitale sommano al capitale) |
| `final_adjustment` | Ultima rata meno il balloon meno la rata ordinaria dell'ultimo periodo |

## Convenzioni (esplicite, riportate in `explain.conventions`)

| Convenzione | Opzioni | Default |
| --- | --- | --- |
| Forma del tasso | (a) tasso annuo con capitalizzazione e periodicità della rata, oppure (b) tasso periodico | esattamente una, obbligatoria |
| Momento di pagamento | rata posticipata: rata $k$ al tempo $k$ | obbligatorio, solo `ordinary` |
| Tipo di ammortamento | `french`, `fixed_amortisation_schedule`, `bullet` | obbligatorio |
| Preammortamento | da 0 a $n - 1$ periodi di soli interessi | 0, riportato come default |
| Balloon | $0 \le B < P$, rimborsato con l'ultima rata | 0, riportato come default |
| Arrotondamento | nessuno, oppure da 0 a 6 decimali con `half_up` o `half_even`, sulla rata (`french`) o sulla quota capitale (`fixed_amortisation_schedule`) | nessuno |
| Interessi di un periodo | tasso periodico × debito residuo iniziale (regime di interesse composto), mai arrotondati | fissa |
| Rettifica dell'ultima rata | ultima quota capitale = debito residuo iniziale dell'ultimo periodo | sempre applicata, riportata |

La tracciabilità del calcolo (audit trail) ha un numero fisso di passaggi qualunque sia la
durata: il tasso periodico, l'importo ordinario e il suo arrotondamento, la ricorrenza
enunciata una volta, il primo periodo (e il primo periodo dopo il preammortamento) con i numeri
sostituiti, l'ultimo periodo con la rettifica, e i totali. Le righe riportano i numeri di ogni
periodo.

## Errori

| Codice | Quando |
| --- | --- |
| `FM-E001` | Momento di pagamento `due`, capitale pari a 0 o negativo, `number_of_instalments` pari a 0 o superiore a 10000, forma di tasso incompleta o doppia, arrotondamento senza modalità o con più di 6 decimali, oppure un campo mancante o sconosciuto |
| `FM-E003` | Il tasso periodico è pari o inferiore a −100% |
| `FM-E020` | Il balloon è negativo, non inferiore al capitale, oppure indicato con `bullet` |
| `FM-E022` | Il preammortamento non è inferiore al numero delle rate, oppure è indicato con `bullet` |
| `FM-E023` | Con l'arrotondamento, il debito residuo diventerebbe negativo prima dell'ultimo periodo, oppure l'ultima quota capitale o l'ultima rata sarebbe negativa (arrotondamento troppo grossolano per il finanziamento) |
| `FM-E008` | Overflow aritmetico (input molto al di fuori di un intervallo realistico) |

## Precisione

Ogni periodo usa una moltiplicazione e due sottrazioni su 28 cifre significative. Un errore
nell'ultima cifra della rata cresce di $(1 + i)$ a ogni periodo, quindi su finanziamenti
realistici le righe coincidono con un calcolo indipendente a 50 cifre entro circa $10^{-22}$
del capitale o meglio; il debito residuo finale è esattamente zero per costruzione. I casi di
riferimento sono verificati a $10^{-20}$, oppure esattamente quando tutti i valori sono decimali
finiti.

## Verificato con

- Due esempi svolti da libri di testo, entro mezzo centesimo (i libri arrotondano ogni voce al
  centesimo): Valerie Watts, *Business and Financial Mathematics* (Fanshawe College
  Pressbooks), sezione 6.1, il finanziamento alle imprese di 50.000; Jean-Paul Olivier,
  *Business Math: A Step-by-Step Handbook* (Lyryx Learning, 2018 Revision A), Example 13.3A.
- Casi di riferimento in `tests/golden/loans.amortisation_schedule/`: gli esempi svolti di
  ECMA-376 della quota capitale e un calcolo indipendente a 50 cifre
  (`scripts/reference_loans.py`), confrontato anche riga per riga su finanziamenti casuali dei
  tre tipi.
- `numpy_financial.ipmt` e `numpy_financial.ppmt` per piani alla francese senza
  arrotondamento.
