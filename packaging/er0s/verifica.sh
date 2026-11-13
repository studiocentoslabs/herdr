#!/usr/bin/env bash
# verifica.sh — prove di un candidato della distribuzione er0s (ER0S.md), da lanciare nella radice del sorgente.
#
#   verifica.sh --plugin DIR [--build-id ID] [--solo-plugin]
#
#   --plugin      cartella del plugin er0s da installare nella prova (plugin/herdr del repo er0s)
#   --build-id    identificativo nella versione (predefinito: SHA corto di HEAD, se c'e' git; altrimenti "locale")
#   --solo-plugin salta lint e test di herdr: build della distribuzione e prova del plugin soltanto
#
# Passi: zig e sue dipendenze, lint di herdr (fmt, clippy), test di herdr (nextest o cargo test, contratti python, bun se c'e'),
# test della distribuzione, build release con ER0S_DISTRIBUTION, prova del plugin in una HOME temporanea con un server
# separato. Ogni passo stampa una riga `passo=<nome> esito=ok|rosso|saltato`; esce 0 solo se nessun passo e' rosso.
# Non tocca la HOME vera, ne' herdr installato: tutto sta in una cartella temporanea tolta all'uscita.
set -uo pipefail

PLUGIN=""; BUILD_ID=""; SOLO_PLUGIN=0
while [ $# -gt 0 ]; do
  case "$1" in
    --plugin) PLUGIN="${2:?manca il valore di --plugin}"; shift 2 ;;
    --build-id) BUILD_ID="${2:?}"; shift 2 ;;
    --solo-plugin) SOLO_PLUGIN=1; shift ;;
    -h|--help) sed -n '2,/^set -uo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "verifica: argomento sconosciuto: $1" >&2; exit 2 ;;
  esac
done
[ -n "$PLUGIN" ] || { echo "verifica: serve --plugin" >&2; exit 2; }
[ -f Cargo.toml ] && [ -f ER0S.md ] || { echo "verifica: lancialo nella radice del fork" >&2; exit 2; }
PLUGIN="$(cd "$PLUGIN" && pwd)"
radice="$(pwd)"
[ -n "$BUILD_ID" ] || BUILD_ID="$(git rev-parse --short=12 HEAD 2>/dev/null || echo locale)"
target="${CARGO_TARGET_DIR:-$radice/target}"
rossi=0

passo() { # passo <nome> <comando...>: esegue, stampa l'esito, conta i rossi
  local nome="$1"; shift
  local inizio=$SECONDS
  if "$@"; then echo "passo=$nome esito=ok secondi=$((SECONDS - inizio))"
  else echo "passo=$nome esito=rosso secondi=$((SECONDS - inizio))"; rossi=$((rossi + 1)); fi
}
saltato() { echo "passo=$1 esito=saltato motivo=$2"; }

zig_ok() { local v; v="$("${ZIG:-zig}" version 2>/dev/null)" || return 1; echo "zig $v"; [[ "$v" == 0.16.* ]]; }
passo zig zig_ok
# Le dipendenze zig di libghostty-vt arrivano dalla rete: si scaricano prima, con tre tentativi, cosi' un timeout
# non diventa un falso rosso di clippy o dei test. ZIG_GLOBAL_CACHE_DIR persistente le scarica una volta sola.
zig_deps() {
  local n; for n in 1 2 3; do
    (cd vendor/libghostty-vt && "${ZIG:-zig}" build --fetch) && return 0
    echo "zig build --fetch: tentativo $n fallito"; sleep $((n * 10))
  done; return 1
}
passo zig-dipendenze zig_deps

if [ "$SOLO_PLUGIN" = 0 ]; then
  passo fmt cargo fmt --check
  passo clippy cargo clippy --all-targets --locked -- -D warnings
  if cargo nextest --version >/dev/null 2>&1; then
    passo test-herdr cargo nextest run --locked --status-level fail --final-status-level fail --failure-output final --success-output never
  else
    passo test-herdr cargo test --locked
  fi
  passo test-contratti python3 -m unittest scripts.test_ui_hot_path_architecture scripts.test_agent_detection_manifest_check \
    scripts.test_changelog scripts.test_config_reference_check scripts.test_docs_translation_parity \
    scripts.test_hermes_integration_asset scripts.test_unix_installer scripts.test_vendor_libghostty_vt \
    scripts.test_vendor_portable_pty
  if command -v bun >/dev/null 2>&1; then
    passo test-bun bun test src/integration/assets/herdr-agent-state.test.ts ./scripts/docs
  else
    saltato test-bun "bun assente"
  fi
fi

passo test-er0s env ER0S_DISTRIBUTION=1 cargo test --locked --bin herdr er0s_distribution
passo build-er0s env ER0S_DISTRIBUTION=1 HERDR_BUILD_CHANNEL=er0s HERDR_BUILD_ID="$BUILD_ID" \
  cargo build --release --locked
binario="$target/release/herdr"

prova_plugin() {
  local tmp; tmp="$(mktemp -d)" || return 1
  # shellcheck disable=SC2064
  trap "rm -rf -- '$tmp'" RETURN
  local ambiente=(env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH -u HERDR_SESSION -u HERDR_CONFIG_PATH -u HERDR_ENV
    -u ER0S_HOME -u ER0S_API_URL HOME="$tmp/home" XDG_CONFIG_HOME="$tmp/config" XDG_STATE_HOME="$tmp/state")
  mkdir -p "$tmp/home" "$tmp/config/er0s-terminal" "$tmp/state"
  printf 'onboarding = false\n\n[update]\nversion_check = false\nmanifest_check = false\n' > "$tmp/config/er0s-terminal/config.toml"
  "${ambiente[@]}" bash "$radice/packaging/er0s/installa.sh" --prefix "$tmp/prefix" --binary "$binario" --plugin "$PLUGIN" || return 1
  "${ambiente[@]}" bash "$radice/packaging/er0s/installa.sh" --prefix "$tmp/prefix" --binary "$binario" --plugin "$PLUGIN" >/dev/null \
    || { echo "seconda installazione fallita"; return 1; }
  local et="$tmp/prefix/bin/er0s-terminal"
  [ "$(ls "$tmp/config")" = "er0s-terminal" ] || { echo "cartelle di config inattese: $(ls "$tmp/config")"; return 1; }
  [ ! -e "$tmp/config/herdr" ] && [ ! -e "$tmp/state/herdr" ] || { echo "creata una cartella herdr"; return 1; }
  local voci; voci="$("${ambiente[@]}" "$et" plugin list --json | grep -o '"er0s"' | wc -l)"
  echo "voci er0s nel registro: $voci"
  local msg; msg="$("${ambiente[@]}" "$et" update 2>&1)" && { echo "update non rifiutato"; return 1; }
  [[ "$msg" == *"does not update from the herdr channel"* ]] || { echo "update: messaggio inatteso: $msg"; return 1; }
  echo "update rifiutato: ok"
  "${ambiente[@]}" "$et" server >"$tmp/server.log" 2>&1 &
  local pid=$!

  for _ in $(seq 1 100); do
    [ -S "$tmp/config/er0s-terminal/herdr.sock" ] && break
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.1
  done
  local esito=0
  if [ -S "$tmp/config/er0s-terminal/herdr.sock" ]; then
    echo "server di prova: socket sotto er0s-terminal"
    "${ambiente[@]}" "$et" status server | sed 's/^/  /' || esito=1
    "${ambiente[@]}" "$et" plugin list --plugin er0s || esito=1
    "${ambiente[@]}" "$et" plugin action list --plugin er0s || esito=1
    "${ambiente[@]}" "$et" server stop >/dev/null || esito=1
  else
    echo "il server di prova non ha aperto il socket"; sed 's/^/  log: /' "$tmp/server.log" | tail -20; esito=1
  fi
  for _ in $(seq 1 50); do kill -0 "$pid" 2>/dev/null || break; sleep 0.1; done
  kill -0 "$pid" 2>/dev/null && { kill "$pid"; echo "server fermato con kill (pid registrato al lancio)"; esito=1; }
  wait "$pid" 2>/dev/null
  return $esito
}
if [ -x "$binario" ]; then passo prova-plugin prova_plugin; else saltato prova-plugin "manca il binario"; rossi=$((rossi + 1)); fi

"$binario" --version 2>/dev/null | sed 's/^/versione: /'
echo "rossi=$rossi"
[ "$rossi" = 0 ]
