# er0s-terminal: distribuzione er0s di herdr

Questo repository è un fork di [herdr](https://github.com/herdrdev/herdr) (Apache-2.0, © gli autori di herdr).
La licenza originale è in `LICENSE`, invariata. herdr è il motore dichiarato di er0s; il nome e il marchio herdr
restano dei loro autori e non sono usati come nostri. Questo fork non è un progetto ufficiale di herdr.

Il ramo `er0s/main` è l'ultima stabile di herdr (tag upstream registrato in `ER0S-BASE`) più una serie corta di
modifiche, una per commit. Ogni file modificato porta un commento che rimanda a questo documento.

## Modifiche rispetto a herdr

Attive solo in una build con la variabile d'ambiente `ER0S_DISTRIBUTION=1`; senza, il binario si comporta come herdr
e i test di herdr girano senza cambiamenti.

| Modifica | File | Perché |
| --- | --- | --- |
| Nome er0s nella finestra, nel titolo predefinito e nel primo avvio; avviso «motore: herdr (Apache-2.0)» nel primo avvio e nell'aiuto | `src/config/window_title.rs`, `src/terminal_effects.rs`, `src/ui/onboarding.rs`, `src/client/shell/sidebar.rs`, `src/client/shell/endpoint_sidebar.rs`, `src/client/shell/overlays.rs` | Distinguere il prodotto dal motore conservandone l'attribuzione; i titoli personalizzati restano configurabili |
| Cartelle di configurazione, stato e socket sotto `er0s-terminal` (`er0s-terminal-dev` in debug) invece di `herdr` | `src/er0s_distribution.rs`, `src/config/io.rs` | Convivere con un herdr installato sulla stessa macchina senza toccarne config, plugin e sessioni |
| Niente aggiornamento dal canale ufficiale herdr (`update` rifiutato, nessun controllo in background) | `src/er0s_distribution.rs`, `src/update.rs` | Un aggiornamento ufficiale sostituirebbe il binario con herdr e toglierebbe la distribuzione er0s; si aggiorna con `riallinea-fork.sh` |
| Installazione come `er0s-terminal` con il plugin er0s registrato | `packaging/er0s/installa.sh` | Il comando `er0s` esiste già (runtime di er0s); il plugin è lo stesso pacchetto usato con herdr ufficiale |
| Riallineamento settimanale a una nuova stabile | `packaging/er0s/riallinea-fork.sh`, `verifica.sh`, `test-noti.txt` | Integrare upstream su un ramo di prova con build, test e prova del plugin prima di toccare `er0s/main` |

Contratto conservato: protocollo e socket API, formato `herdr-plugin.toml`, variabili `HERDR_*`, nome del pacchetto
Cargo. Il binario costruito si chiama ancora `herdr` dentro `target/`; prende il nome `er0s-terminal` solo
all'installazione.

## Avvertenze

- Dentro un pannello di herdr ufficiale le variabili `HERDR_*` puntano al server ufficiale: `installa.sh` le toglie
  prima di chiamare il binario; chi lancia `er0s-terminal` a mano da un pannello herdr deve fare lo stesso.
- Non coperti: risoluzione del binario sulle macchine remote (`--remote`), installer pubblico, aggiornamento firmato.

## Agenti nelle impostazioni della TUI

Patch del 15/11/2026: `src/er0s_agents.rs` e `src/client/shell/{state,settings,settings_overlay}.rs`.
Nella build er0s, la finestra delle impostazioni include **er0s · agenti**. Tab cambia sezione, ↑/↓ sceglie
il campo del ruolo, Invio apre l'elenco di CLI o modelli, Invio salva; Esc torna ai campi o chiude.
Il file non viene creato finché non salvi. Il cambio di CLI sceglie anche il default del ruolo se torni alla sua CLI iniziale, altrimenti il primo modello del suo elenco;
puoi poi scegliere un altro modello nell'elenco. Nessuna CLI viene eseguita dalle impostazioni.

Il contratto con il launcher del plugin è `er0s.toml` nella cartella di configurazione della TUI:
`~/.config/er0s-terminal/er0s.toml` in release (`er0s-terminal-dev` in debug; segue `XDG_CONFIG_HOME`). `ER0S_TUI_CONFIG` permette a TUI e launcher di usare un percorso esplicito comune; un profilo con HOME/XDG separati deve impostarlo per entrambi.
La TUI scrive solo quel file e non riavvia i pannelli. Il launcher deve rileggerlo al prossimo avvio di ciascun
pannello. Il formato coincide con il launcher installato su soliso il 15/11: `[ruoli.ceo/cfo/cbo/clo]` e `[esecutori.codice]`, campi `cli` e `modello`. Quel launcher oggi avvia soltanto Claude; l’avvio delle altre CLI e l’esecutore richiedono l’adeguamento del plugin/launcher nel repository er0s, fuori da questa patch del fork.

```toml
[ruoli.ceo]
cli = "claude"
modello = "opus"

[ruoli.cfo]
cli = "claude"
modello = "sonnet"

[ruoli.cbo]
cli = "claude"
modello = "sonnet"

[ruoli.clo]
cli = "claude"
modello = "sonnet"

[esecutori.codice]
cli = "codex"
modello = "default"
```

Questi sono anche i default quando manca il file o una tabella di ruolo. Una tabella presente deve avere
entrambi i campi stringa `cli` e `modello`. `default` significa che il launcher non passa `--model` alla CLI.
Il catalogo locale della TUI è definito da `models()` in `src/er0s_agents.rs`: alias opus/sonnet/haiku per Claude,
gpt-6.1-sol/gpt-6-astra/gpt-6.1-terra per Codex, un elenco ristretto per OpenCode e agy, oltre a `default`.
`default` è disponibile per Codex/OpenCode/agy; Claude usa gli alias espliciti opus/sonnet/haiku. Le scelte sono preferenze; l'accesso al modello dipende dall'account della CLI. Il catalogo non richiede chiamate di rete.

CLI, modello, ruolo o TOML non validi vengono segnalati senza sovrascrivere il file; dopo averlo corretto puoi
premere Invio per rileggerlo. Il salvataggio sostituisce atomicamente il file con permessi privati e conserva
le altre tabelle e gli altri ruoli, ma riformatta il TOML e non conserva i commenti. Un percorso symlink viene rifiutato.
Nessun campo del protocollo, motore o registro plugin cambia; licenza e attribuzione originali restano intatte.

La prova nativa ha inoltre mostrato un test instabile già esistente, `agent_start_rejects_a_shell_replaced_by_a_foreground_program` (`tests/cli/agents.rs`): rosso nella suite, verde da solo. La fixture ora osserva il PID del programma Python nel foreground tramite l’API e lo mantiene attivo fino alla pulizia, invece di aspettare 150 ms con un processo che termina dopo cinque secondi. Le asserzioni sull’errore `agent_pane_busy` e sulla topologia restano invariate; nessuna modifica al motore, nessun nuovo test escluso.


## Complessità nelle impostazioni

Aggiunta del 15/11/2026, ordine del direttore delle 17:00: accanto a **er0s · agenti** compare la sezione
**complessità**. Selezioni semplice, media o difficile con ↑/↓, Invio apre l’elenco dei modelli Claude,
Invio salva; Esc torna indietro senza salvare la scelta aperta. Nei terminali stretti la barra scorre per
mantenere visibile la sezione attiva. Il file e il percorso sono gli stessi delle preferenze per ruolo.

```toml
[complessita]
semplice = "haiku"
media = "sonnet"
difficile = "opus"
```

Questi sono i default anche per una singola voce mancante. CLI o modello per ruolo non vengono modificati:
il salvataggio della complessità conserva `[ruoli.*]`, `[esecutori.*]`, `[compiti.*]` e le altre tabelle,
comprese le configurazioni parziali di ruolo ammesse dal lettore del plugin. Un livello sconosciuto, un tipo
errato o un modello fuori dall’elenco opus/sonnet/haiku vengono rifiutati senza sostituire il file.

Il consumer è `plugin/herdr/first_run.py` della PR Autuori-Holding/er0s#2514 (`selected_model`):
**modello del singolo compito > complessità del compito > ruolo**. La TUI cambia solo le tre preferenze,
non classifica i compiti e non modifica il lanciatore né i pannelli già aperti. Un override in `[compiti.*]`
resta prioritario; se la complessità manca, rimane il valore per ruolo.
