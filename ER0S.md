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
