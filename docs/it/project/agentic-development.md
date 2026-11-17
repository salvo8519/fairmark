---
title: Sviluppo agentico
description: Come Fairmark viene costruito da agenti AI e da un maintainer — ruoli,
  memoria, corsie degli sprint e i controlli deterministici che fanno rispettare ogni
  regola.
source: project/agentic-development.md
source_hash: f22a88b74fb6
---
# Sviluppo agentico

Fairmark è sviluppato da agenti AI di programmazione che lavorano con un maintainer. Il motore
segue uno standard rigoroso (decimali esatti, golden case da fonti indipendenti, contratti
versionati), e lo stesso vale per il processo che lo costruisce. Un principio attraversa tutta
la pagina: **una regola che conta è fatta rispettare da un controllo che può fallire, non dalla
buona volontà di un agente.** Dove una regola è ancora solo scritta, questa pagina lo dice.

```mermaid
flowchart LR
    R[Roadmap<br/>roadmap.yaml] --> S[Sprint<br/>corsie con percorsi]
    S --> W[Un worktree<br/>per issue]
    W --> P[Spec, approvata<br/>per i metodi]
    P --> T[Prima i test]
    T --> I[Implementazione]
    I --> V[Revisione<br/>per livello di rischio]
    V --> Q[Pull request]
    Q --> G[Controlli<br/>della CI]
    G --> M[Il maintainer<br/>esegue il merge]
    M --> R
    classDef plan fill:#0B1F3A,color:#F7F8FA,stroke:#0B1F3A
    classDef gate fill:#12325C,color:#F7F8FA,stroke:#12325C
    classDef work fill:#F7F8FA,color:#0B1F3A,stroke:#475569
    class R,S,M plan
    class G,V gate
    class W,P,T,I,Q work
```

## Una sola fonte, copie generate

I ruoli e le skill degli agenti sono scritti una volta sola, in
[`.agents/`](https://github.com/salvo8519/fairmark/tree/main/.agents), in formati aperti (Agent
Skills, definizioni dei ruoli in Markdown). Ogni sistema di agenti legge una copia generata da
quella fonte: oggi `.claude/`, e le strutture degli altri sistemi man mano che vengono aggiunte
(#328). Il contratto comune a tutti gli agenti è
[`AGENTS.md`](https://github.com/salvo8519/fairmark/blob/main/AGENTS.md), che Codex, Cursor,
Copilot e altri leggono in modo nativo.

| Regola | Fatta rispettare da |
| --- | --- |
| Le copie non si modificano mai a mano | `scripts/sync_agents.py --check` nella CI (file generati aggiornati) |
| Un nuovo sistema di agenti è una modifica del generatore, non una copia | lo stesso controllo: un file in una copia senza fonte fa fallire la CI |

## Ruoli e skill

Dodici ruoli, ciascuno con una responsabilità ristretta e gli strumenti che gli servono:

| Ruolo | Responsabilità | Scrive |
| --- | --- | --- |
| `architect` | Decisioni strutturali, ADR | Decisioni |
| `spec-writer` | Specifiche, piani, elenchi dei compiti | Spec |
| `test-engineer` | Golden case e test, prima del codice | Test |
| `core-engineer` | Il core in Rust, da una spec approvata | Codice |
| `docs-writer` | Pagine di metodologia, guide, riferimenti | Documentazione |
| `translator` | L'edizione italiana, con il glossario | Traduzioni |
| `code-reviewer` | Il diff rispetto alla costituzione e alla spec | Niente (sola lettura) |
| `numerical-auditor` | Ricalcolo indipendente dei risultati | Niente (sola lettura) |
| `security-reviewer` | Dipendenze, workflow, gestione dei dati | Niente (sola lettura) |
| `ci-investigator` | Causa di una CI rossa | Niente (sola lettura) |
| `release-manager` | Preparazione e verifica dei rilasci | File di rilascio |
| `sprint-planner` | Il prossimo sprint e le sue corsie | File degli sprint |

Le skill raccolgono i flussi di lavoro che i ruoli seguono: `session-start` e `session-end`,
`spec-new`, `implement-spec`, `add-method`, `golden-case`, `pre-pr`, `pr-triage`,
`plan-sprint`, `run-sprint`, `roadmap-update`, `release` e altre. La sessione principale è
l'orchestratore: delega ai ruoli e mantiene la visione d'insieme, perché un ruolo non può
avviarne un altro.

## Memoria su tre livelli

Gli agenti dimenticano tutto tra una sessione e l'altra. Ciò che sopravvive è conservato come
testo semplice, nel posto adatto a quanto deve durare
([ADR 0015](https://salvo8519.github.io/fairmark/project/adr/0015-agent-memory-one-source-tiered-mirrors/)):

| Livello | Dura | Dove | Come è condiviso |
| --- | --- | --- | --- |
| Esecuzione | Da secondi a ore | Stato dello sprint, un file YAML per issue | Nella directory git comune a tutti i worktree; mai salvato nel repository |
| Passaggio di consegne | Giorni | Il diario delle sessioni, `.agents/journal/` | Salvato con la pull request della sessione |
| Piano | Permanente | [`roadmap/roadmap.yaml`](https://github.com/salvo8519/fairmark/blob/main/roadmap/roadmap.yaml): milestone, epic, attività | Salvato nel repository; sincronizzato con le issue di GitHub |

Nessun database: ogni livello è leggibile da qualsiasi agente di qualsiasi sistema, e
verificabile in un diff. Un crash o la perdita del contesto costano minuti:
`scripts/project_state.py` mostra a che punto è il progetto, il diario dice dove si è fermata
l'ultima sessione, e il branch su GitHub contiene il lavoro.
`scripts/journal.py search <issue|area|data|testo>` trova le voci su una issue, prima le sue;
`session-start` le cita per ogni issue che propone. L'indice è costruito dalle voci ogni volta
che viene letto e non viene mai salvato nel repository, quindi le corsie parallele non vanno
mai in conflitto su di esso.

## Sprint e corsie

Uno sprint raggruppa le issue in corsie che procedono in parallelo, ciascun agente nel proprio
worktree git ([sprints/README.md](https://github.com/salvo8519/fairmark/blob/main/sprints/README.md)).
Le corsie non devono toccare gli stessi file, e questo viene verificato, non dato per scontato:

- ogni corsia dichiara `paths`, i pattern glob dei file che può modificare;
- ogni gruppo di file generati (`registry`: schemi, riferimento dei metodi, blocchi del README,
  esempi dei tool; `agents`: la copia `.claude/`) appartiene al massimo a una corsia, che lo
  indica in `generates`;
- `scripts/sprint.py validate` rifiuta le corsie sovrapposte, indicando le corsie e un percorso
  di esempio;
- su una pull request di sprint, la CI esegue `scripts/sprint.py check-diff` e fallisce se un
  file modificato è fuori dalla corsia della issue.

## Controlli

Ogni pull request passa gli stessi controlli deterministici, nella CI e, dove costa poco, prima
di un commit. Quelli obbligatori bloccano il merge.

| Regola | Fatta rispettare da |
| --- | --- |
| Nomi di branch, commit e pull request; una issue collegata | `scripts/conventions.py` (igiene delle PR) |
| Una issue chiusa da una pull request è nella roadmap; ogni spec è pianificata | `scripts/roadmap.py check` |
| Le modifiche al codice arrivano con la loro documentazione | `scripts/docs_sync.py impact`, anche come hook di fine sessione dell'agente |
| Ogni metodo ha una pagina, una voce di navigazione, golden case e un helper Python | `scripts/docs_sync.py coverage` |
| Le pagine pubbliche descrivono solo ciò che esiste | `scripts/docs_sync.py claims` |
| Le pagine italiane seguono la fonte inglese | `scripts/i18n.py check` |
| Le versioni precedenti degli schemi non cambiano mai; le modifiche sono versionate | `scripts/schema_snapshots.py --check` e il job dei contratti |
| I golden case esistenti cambiano solo con una modifica numerica revisionata | un hook sulle modifiche in ogni worktree e `scripts/check_golden_changes.py` |
| I test del core si accorgono dei risultati sbagliati | test di mutazione sulle righe modificate |
| Il server MCP funziona per un client reale, con schemi dei tool portabili | `scripts/mcp_protocol_check.py` con l'MCP Inspector |
| Ogni action fissata a un commit; nessuna icona in nessun file | `scripts/pin_actions.py --check`, `scripts/check_style.py` |
| Ogni script di processo ha un test | `python/tests/scripts/test_every_script_is_tested.py` |

## Sforzo proporzionato al rischio

Le revisioni fatte dagli agenti costano, quindi il loro numero segue il rischio della modifica.
Ogni pull request dichiara il proprio livello
([AGENTS.md](https://github.com/salvo8519/fairmark/blob/main/AGENTS.md), Review standards):

| Livello | Modifiche | Revisione prima del merge |
| --- | --- | --- |
| Small | Documentazione, roadmap, strumenti, CI, correzioni che non cambiano numeri né contratti | I controlli; nessun agente di revisione |
| Substantial | Logica dei metodi, contratti, adattatori | Un passaggio del `code-reviewer` |
| Numerical change | Cambiano risultati o golden case | `code-reviewer` e `numerical-auditor` |

Ogni rilievo di una revisione si corregge nella stessa pull request. Un difetto non si rinvia
mai.

## Cosa decide il maintainer

Gli agenti propongono, il maintainer decide. Il maintainer approva le spec e le scelte di
metodo, approva ogni modifica a un golden case esistente, decide la roadmap, approva lo sprint,
esegue i merge e pubblica i rilasci. Queste decisioni sono registrate dove gli agenti le
leggono: nella spec, nella issue, nel diario o in un ADR.

## Cosa non è ancora verificato

- Burn-down degli sprint e metriche del ciclo (tempo dal branch al merge, controlli falliti,
  passaggi degli agenti per issue) sono pianificati nella #320; fino ad allora il costo del
  processo non è misurato.
- Le copie per sistemi di agenti diversi da Claude Code sono pianificate nella #328.
