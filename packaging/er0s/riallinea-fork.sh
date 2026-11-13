#!/usr/bin/env bash
# riallinea-fork.sh — ricetta settimanale della distribuzione er0s (ER0S.md): porta una stabile di herdr su un ramo di
# prova sopra er0s/main, lo prova e scrive l'esito in un file. Non tocca er0s/main e non fa push.
#
#   riallinea-fork.sh --plugin DIR --esito FILE [--tag TAG] [--lavoro DIR] [--esegui COMANDO] [--zig PATH] [--zig-cache DIR]
#
#   --plugin  cartella del plugin er0s (plugin/herdr del repo er0s) da provare con il candidato
#   --esito   file in cui scrivere l'esito (sovrascritto)
#   --tag     tag upstream da integrare (predefinito: l'ultima stabile vX.Y.Z; le preview non si prendono mai da sole)
#   --lavoro  cartella in cui creare il worktree di prova (predefinito: accanto al checkout, <checkout>-riallinea)
#   --esegui  comando che esegue le prove in un worktree, chiamato come `COMANDO <worktree> <comando...>`
#             (es. uno script che costruisce su un'altra macchina); predefinito: in locale
#   --zig     percorso di zig 0.16 sulla macchina delle prove, passato a verifica.sh come ZIG
#   --zig-cache cartella persistente della cache zig sulla macchina delle prove (ZIG_GLOBAL_CACHE_DIR)
#
# Passi: fetch di upstream e dei tag; base attuale da ER0S-BASE su er0s/main; misura (commit e percorsi cambiati
# upstream, percorsi toccati dalle patch er0s, intersezione); ramo er0s/riallinea-<tag> in un worktree; merge del tag
# (se la base e' gia' quella, nessun merge: si riprovano build e plugin); in caso di conflitti li elenca, lascia il
# worktree com'e' per risolverli ed esce 1; altrimenti commit, verifica.sh e verdetto.
# Uscita: 0 candidato verde, 1 conflitti o prove rosse, 2 uso sbagliato.
# Dopo un verde: revisione del diff, poi `git switch er0s/main && git merge --ff-only er0s/riallinea-<tag>` e push al fork.
set -uo pipefail

die() { echo "riallinea: $*" >&2; exit 2; }
PLUGIN=""; ESITO=""; TAG=""; LAVORO=""; ESEGUI=""; ZIG_PATH=""; ZIG_CACHE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --plugin) PLUGIN="${2:?manca il valore di --plugin}"; shift 2 ;;
    --esito) ESITO="${2:?manca il valore di --esito}"; shift 2 ;;
    --tag) TAG="${2:?}"; shift 2 ;;
    --lavoro) LAVORO="${2:?}"; shift 2 ;;
    --esegui) ESEGUI="${2:?}"; shift 2 ;;
    --zig) ZIG_PATH="${2:?}"; shift 2 ;;
    --zig-cache) ZIG_CACHE="${2:?}"; shift 2 ;;
    -h|--help) sed -n '2,/^set -uo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) die "argomento sconosciuto: $1 (--help)" ;;
  esac
done
[ -n "$PLUGIN" ] && [ -n "$ESITO" ] || die "servono --plugin e --esito (--help)"
[ -f "$PLUGIN/herdr-plugin.toml" ] || die "manca $PLUGIN/herdr-plugin.toml"
PLUGIN="$(cd "$PLUGIN" && pwd)"
checkout="$(git rev-parse --show-toplevel 2>/dev/null)" || die "lancialo dentro il checkout del fork"
cd "$checkout" || die "checkout irraggiungibile"
git remote get-url upstream >/dev/null 2>&1 || die "manca il remote upstream (herdrdev/herdr)"
git rev-parse --verify -q er0s/main >/dev/null || die "manca il ramo er0s/main"
mkdir -p "$(dirname "$ESITO")"; : > "$ESITO" || die "non posso scrivere $ESITO"

scrivi() { printf '%s\n' "$*" | tee -a "$ESITO"; }
inizio=$SECONDS
scrivi "# riallineamento er0s-terminal $(date -u +%Y-%m-%dT%H:%M:%SZ)"

git fetch -q upstream --tags || { scrivi "esito=rosso motivo=fetch-upstream"; exit 1; }
base_tag="$(git show er0s/main:ER0S-BASE | sed -n 's/^upstream_tag=//p')"
base_commit="$(git show er0s/main:ER0S-BASE | sed -n 's/^upstream_commit=//p')"
[ -n "$base_commit" ] || { scrivi "esito=rosso motivo=ER0S-BASE-illeggibile"; exit 1; }
[ -n "$TAG" ] || TAG="$(git tag -l 'v[0-9]*' | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1)"
target_commit="$(git rev-parse -q --verify "${TAG}^{commit}")" || { scrivi "esito=rosso motivo=tag-inesistente tag=$TAG"; exit 1; }
er0s_sha="$(git rev-parse er0s/main)"
if [ "$target_commit" != "$base_commit" ] && git merge-base --is-ancestor "$target_commit" "$base_commit"; then
  scrivi "esito=rosso motivo=target-piu-vecchio-della-base tag=$TAG base=$base_tag"; exit 1
fi
scrivi "base=$base_tag $base_commit"
scrivi "target=$TAG $target_commit"
scrivi "er0s_main=$er0s_sha"

# Misura prima dell'integrazione: quanto cambia upstream e dove tocca le patch er0s.
lista_dir="$(mktemp -d)"; trap 'rm -rf -- "$lista_dir"' EXIT
git diff --no-renames --name-only "$base_commit" "$target_commit" | sort > "$lista_dir/upstream"
git diff --no-renames --name-only "$base_commit" er0s/main | sort > "$lista_dir/er0s"
comm -12 "$lista_dir/upstream" "$lista_dir/er0s" > "$lista_dir/comuni"
read -r indietro avanti < <(git rev-list --left-right --count "${base_commit}...${target_commit}")
scrivi "commit_upstream=$avanti nuovi, $indietro solo nella base"
scrivi "percorsi_upstream_cambiati=$(wc -l < "$lista_dir/upstream")"
scrivi "percorsi_patch_er0s=$(wc -l < "$lista_dir/er0s")"
scrivi "percorsi_in_comune=$(wc -l < "$lista_dir/comuni")"
sed 's/^/  in_comune: /' "$lista_dir/comuni" | tee -a "$ESITO"

ramo="er0s/riallinea-$TAG"
[ -n "$LAVORO" ] || LAVORO="$checkout-riallinea"
wt="$LAVORO/er0s-terminal-$TAG"
git rev-parse -q --verify "$ramo" >/dev/null && { scrivi "esito=rosso motivo=ramo-gia-esistente ramo=$ramo (rimuovilo o usa un altro tag)"; exit 1; }
[ -e "$wt" ] && { scrivi "esito=rosso motivo=worktree-gia-esistente path=$wt"; exit 1; }
mkdir -p "$LAVORO"
git worktree add -q -b "$ramo" "$wt" er0s/main || { scrivi "esito=rosso motivo=worktree"; exit 1; }
scrivi "worktree=$wt ramo=$ramo"

if [ "$target_commit" = "$base_commit" ]; then
  scrivi "merge=nessuno (base gia' su $TAG: si riprovano build e plugin)"
else
  if git -C "$wt" merge -q --no-ff --no-commit "$target_commit" >/dev/null 2>&1; then
    printf 'upstream_tag=%s\nupstream_commit=%s\n' "$TAG" "$target_commit" > "$wt/ER0S-BASE"
    git -C "$wt" add ER0S-BASE
    git -C "$wt" commit -q -m "chore(er0s): riallinea a herdr $TAG" || { scrivi "esito=rosso motivo=commit-merge"; exit 1; }
    scrivi "merge=pulito"
  else
    scrivi "merge=conflitti"
    git -C "$wt" diff --name-only --diff-filter=U | sed 's/^/  conflitto: /' | tee -a "$ESITO"
    scrivi "esito=rosso motivo=conflitti: risolvili in $wt mantenendo il contratto pubblico e le patch di ER0S.md, poi riprova con verifica.sh"
    scrivi "durata_secondi=$((SECONDS - inizio))"
    exit 1
  fi
fi
candidato="$(git -C "$wt" rev-parse HEAD)"
scrivi "candidato=$candidato"

# Il plugin entra nel worktree come copia non tracciata, cosi' arriva anche a una macchina remota delle prove.
grep -qx '/.er0s-prova/' "$(git rev-parse --git-common-dir)/info/exclude" 2>/dev/null \
  || echo '/.er0s-prova/' >> "$(git rev-parse --git-common-dir)/info/exclude"
mkdir -p "$wt/.er0s-prova"
tar -C "$PLUGIN" --exclude=.git --exclude=__pycache__ --exclude='*.pyc' -cf - . | (mkdir -p "$wt/.er0s-prova/plugin" && tar -C "$wt/.er0s-prova/plugin" -xf -)

comando=(bash packaging/er0s/verifica.sh --plugin .er0s-prova/plugin --build-id "${candidato:0:12}")
[ -n "$ZIG_PATH" ] && comando=(env "ZIG=$ZIG_PATH" "${comando[@]}")
[ -n "$ZIG_CACHE" ] && comando=(env "ZIG_GLOBAL_CACHE_DIR=$ZIG_CACHE" "${comando[@]}")
registro="${ESITO%.*}.log"
scrivi "## prove (registro completo: $registro)"
if [ -n "$ESEGUI" ]; then
  # shellcheck disable=SC2086 # COMANDO puo' avere argomenti propri
  $ESEGUI "$wt" "${comando[@]}" > "$registro" 2>&1
else
  (cd "$wt" && "${comando[@]}") > "$registro" 2>&1
fi
stato=$?
grep -E '^(passo=|versione:|rossi=|voci |update |server |zig |  )' "$registro" | tee -a "$ESITO"
scrivi "durata_secondi=$((SECONDS - inizio))"
if [ "$stato" = 0 ]; then
  scrivi "esito=verde candidato=$candidato ramo=$ramo"
  scrivi "prossimo: revisione del diff, poi git switch er0s/main && git merge --ff-only $ramo && git push origin er0s/main"
  exit 0
fi
scrivi "esito=rosso motivo=prove candidato=$candidato (dettagli: rilancia verifica.sh in $wt)"
exit 1
