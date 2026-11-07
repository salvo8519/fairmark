---
source: concepts/analysis.md
source_hash: 39b0c4ec36f5
---
# Metodi di analisi

Una valutazione è una stima puntuale costruita su ipotesi incerte. I revisori pongono sempre le
stesse domande: quale tasso di attualizzazione giustifica questo prezzo, come varia il valore
al variare delle ipotesi, qual è il valore in uno scenario sfavorevole. I metodi di analisi
rispondono all'interno del motore, con ogni valutazione riproducibile e ogni convenzione
riportata.

Un metodo di analisi non valuta nulla da solo. Valuta ripetutamente un altro metodo registrato,
il metodo interno, modificandone uno o più input decimali, e ne riporta gli output.

| Metodo | Stato | Domanda a cui risponde |
| --- | --- | --- |
| [`analysis.goal_seek`](../methods/analysis-goal-seek.md) | Disponibile | Quale valore di un input porta un output a un valore obiettivo |

Tabelle di sensitività, scenari e tornado sono specificati nella SPEC-0008 e seguiti nella
[roadmap](https://salvo8519.github.io/fairmark/project/roadmap/) (in inglese); ogni metodo del tema `analysis` segue le regole di questa
pagina.

## Composizione tramite il registro

Il metodo interno è scelto dal chiamante al momento dell'esecuzione, tramite il suo
identificativo: `"method": "dcf.fcff"`. L'analisi passa `base_input` al registro all'interno di
`fairmark-core`, esattamente come un adattatore lo passerebbe a `run`. Il metodo interno
interpreta, convalida e calcola come sempre, quindi si applicano il suo schema e i suoi errori
(ADR 0010). Ne consegue che:

- qualsiasi metodo può essere usato come metodo interno, compresi quelli aggiunti in seguito,
  senza modifiche all'analisi;
- lo schema di un metodo di analisi descrive `base_input` solo come oggetto; è il metodo interno
  a convalidarlo al momento dell'esecuzione. Gli schemi delle analisi non cambiano quando cambia
  un metodo interno;
- ogni risultato registra in `inner_method` il metodo interno e la versione eseguita da questo
  motore. Per riprodurre un risultato servono quella versione e la versione del motore indicata
  in `meta`;
- la composizione ha un solo livello: un metodo di analisi non può valutare un altro metodo di
  analisi (`FM-E017`).

## Puntatori

Le variabili e gli output sono identificati da JSON Pointer (RFC 6901), la notazione standard
per indicare una posizione in un documento JSON:

| Puntatore | Si riferisce a |
| --- | --- |
| `/discount_rate` | il membro `discount_rate` |
| `/free_cash_flows/0` | il primo elemento della lista `free_cash_flows` |
| `/a~1b` | il membro `a/b` (`~1` sta per `/`, `~0` per `~`) |

Il puntatore di una variabile si risolve in `base_input`; il puntatore di un output si risolve
nel `value` del risultato interno. Entrambi devono iniziare con `/` e condurre a un decimale
esistente, espresso come numero JSON o come stringa interpretabile come decimale. Un puntatore
che non si risolve, che usa il token `-` (oltre la fine di una lista) o che conduce a un testo,
una lista o un oggetto restituisce `FM-E015` con l'indicazione del puntatore. I puntatori delle
variabili sono verificati prima di qualsiasi valutazione, quelli degli output alla prima.

Possono variare solo gli input decimali. Sono esclusi gli interi, come il numero di periodi,
le scelte, come `timing`, e le liste intere.

## Le variabili sono scritte come stringhe decimali

Quando l'analisi imposta una variabile, scrive il decimale come stringa JSON nella posizione
indicata dal puntatore e lascia invariati tutti gli altri membri di `base_input`. Le stringhe
mantengono il valore esatto: non interviene la virgola mobile binaria. Un input base che
esprime un valore come numero `0.1` o come stringa `"0.1"` produce un `value` identico;
`meta.input_hash` è diverso, perché l'hash copre la richiesta così come è scritta.

## Costo limitato

Una richiesta esegue al massimo 10.000 valutazioni del metodo interno. Quando il numero di
valutazioni è noto in anticipo (una tabella di 101 × 100 celle), una richiesta oltre il limite
restituisce `FM-E016` prima della prima valutazione. La ricerca obiettivo (goal seek) richiede
al massimo 2 + `max_iterations` valutazioni, e `max_iterations` è limitato a 1.000.

## Determinismo

Le valutazioni sono eseguite una dopo l'altra in un ordine fisso e documentato: gli estremi
dell'intervallo di ricerca e poi le iterazioni nella ricerca obiettivo; prima le righe e poi le
colonne in una tabella; scenari e variabili del tornado nell'ordine di dichiarazione, il valore
basso prima di quello alto. Lo stesso input e le stesse versioni producono lo stesso output fino
all'ultima cifra, compreso l'ordine di `explain.steps`.

`meta.input_hash` è calcolato con le chiavi di `base_input` ordinate ricorsivamente, quindi due
richieste che differiscono solo per l'ordine delle chiavi hanno lo stesso hash. Vedi
[il contratto di determinismo](determinism.md).

## Errori del metodo interno

Un errore sollevato dal metodo interno mantiene il proprio codice. Nella ricerca obiettivo e nel
tornado qualsiasi errore di questo tipo interrompe la richiesta, perché la ricerca o
l'ordinamento non sono definiti senza quel valore. Nelle tabelle di sensitività e negli scenari
una cella o uno scenario in errore registra il proprio errore e gli altri sono comunque
restituiti: una tabella WACC × g contiene naturalmente celle in cui la crescita non è inferiore
al tasso di attualizzazione (`FM-E004`). Un difetto interno (`FM-E012`) interrompe sempre la
richiesta.
