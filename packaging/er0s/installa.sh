#!/usr/bin/env bash
# installa.sh — installa la distribuzione er0s (ER0S.md): il binario come er0s-terminal e il plugin er0s registrato.
#
#   installa.sh --prefix DIR --binary PATH --plugin DIR
#
#   --prefix  cartella d'installazione: il binario va in DIR/bin/er0s-terminal, il plugin in
#             DIR/share/er0s-terminal/plugins/er0s (copia, cosi' non dipende dal checkout da cui arriva)
#   --binary  binario costruito con ER0S_DISTRIBUTION=1 (target/release/herdr); si rifiuta un herdr qualunque
#   --plugin  cartella del plugin er0s (plugin/herdr nel repo er0s), con herdr-plugin.toml e id = "er0s"
#             Se accanto c'è il pacchetto pubblico plugin/er0s, include manifest e skill dei ruoli.
#
# Rilanciato con gli stessi argomenti lascia lo stesso risultato. Non scrive mai un file chiamato herdr e non tocca
# le cartelle di herdr: config e registro plugin vanno sotto er0s-terminal (XDG_CONFIG_HOME/er0s-terminal se definita).
# Le variabili HERDR_* di un pannello herdr ufficiale vengono tolte: altrimenti il binario parlerebbe a quel server.
set -euo pipefail

die() { echo "installa: $*" >&2; exit 2; }
PREFIX=""; BIN=""; PLUGIN=""
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) PREFIX="${2:?manca il valore di --prefix}"; shift 2 ;;
    --binary) BIN="${2:?manca il valore di --binary}"; shift 2 ;;
    --plugin) PLUGIN="${2:?manca il valore di --plugin}"; shift 2 ;;
    -h|--help) sed -n '2,/^set -euo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) die "argomento sconosciuto: $1 (--help)" ;;
  esac
done
[ -n "$PREFIX" ] && [ -n "$BIN" ] && [ -n "$PLUGIN" ] || die "servono --prefix, --binary e --plugin (--help)"
[ -x "$BIN" ] || die "binario non eseguibile: $BIN"
[ -f "$PLUGIN/herdr-plugin.toml" ] || die "manca $PLUGIN/herdr-plugin.toml"
grep -Eq '^id[[:space:]]*=[[:space:]]*"er0s"' "$PLUGIN/herdr-plugin.toml" || die "il manifest in $PLUGIN non ha id = \"er0s\""

pulito() { env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH -u HERDR_SESSION -u HERDR_CONFIG_PATH -u HERDR_ENV "$@"; }
versione="$(pulito "$BIN" --version)"
[[ "$versione" == *-er0s* ]] || die "il binario non e' una build er0s (versione: $versione); costruiscilo con ER0S_DISTRIBUTION=1 HERDR_BUILD_CHANNEL=er0s"

mkdir -p "$PREFIX/bin" "$PREFIX/share/er0s-terminal/plugins"
PREFIX="$(cd "$PREFIX" && pwd)"
dest_bin="$PREFIX/bin/er0s-terminal"
tmp_bin="$(mktemp "$PREFIX/bin/.er0s-terminal.XXXXXX")"
cp -- "$BIN" "$tmp_bin" && chmod 755 "$tmp_bin" && mv -f -- "$tmp_bin" "$dest_bin"

dest_plugin="$PREFIX/share/er0s-terminal/plugins/er0s"
tmp_plugin="$(mktemp -d "$PREFIX/share/er0s-terminal/plugins/.er0s.XXXXXX")"
trap 'rm -rf -- "$tmp_plugin"' EXIT
tar -C "$PLUGIN" --exclude=.git --exclude=__pycache__ --exclude='*.pyc' -cf - . | tar -C "$tmp_plugin" -xf -
cli_source="$PLUGIN/../er0s"
if [ -f "$cli_source/.claude-plugin/plugin.json" ] && [ -f "$cli_source/.codex-plugin/plugin.json" ]; then
  [ -d "$cli_source/skills" ] || die "mancano le skill del pacchetto CLI"
  mkdir -- "$tmp_plugin/cli"
  cp -R -- "$cli_source/.claude-plugin" "$cli_source/.codex-plugin" "$cli_source/skills" "$tmp_plugin/cli/"
fi
chmod 755 "$tmp_plugin"
if [ -d "$dest_plugin" ]; then
  vecchio="$dest_plugin.vecchio.$$"; mv -- "$dest_plugin" "$vecchio"; mv -- "$tmp_plugin" "$dest_plugin"; rm -rf -- "$vecchio"
else
  mv -- "$tmp_plugin" "$dest_plugin"
fi
trap - EXIT

pulito "$dest_bin" plugin link "$dest_plugin" >/dev/null
elenco="$(pulito "$dest_bin" plugin list --json)"
printf '%s' "$elenco" | grep -q '"er0s"' || die "il plugin er0s non risulta registrato dopo il link"
echo "installato: $dest_bin ($versione)"
echo "plugin er0s registrato da: $dest_plugin"
echo "config: $(pulito "$dest_bin" plugin config-dir er0s)"
