#!/usr/bin/env bash
# Place le sidecar Typst attendu par Tauri dans src-tauri/binaries/.
#
# La version est épinglée : la pagination d'un livre dépend du moteur qui l'a composé.
# Deux versions de Typst ne rendent pas forcément le même nombre de pages, donc pas le
# même dos. Relever cette version est un changement délibéré, à revalider sur un
# manuscrit réel — pas une mise à jour de routine.
set -euo pipefail

VERSION="0.15.1"
ICI="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ICI/src-tauri/binaries"

triple_hote() {
  local arch os
  arch="$(uname -m)"
  case "$(uname -s)" in
    Darwin) os="apple-darwin" ;;
    Linux)  os="unknown-linux-musl" ;;
    *)      echo "hôte non reconnu : $(uname -s)" >&2; exit 1 ;;
  esac
  [ "$arch" = "arm64" ] && arch="aarch64"
  echo "${arch}-${os}"
}

usage() {
  cat >&2 <<'FIN'
usage : typst.sh [--local] [triple]

  (défaut)   télécharge la version épinglée pour le triple demandé
  --local    reprend le typst du PATH, à condition qu'il porte la version épinglée
  triple     cible Rust (défaut : celle de l'hôte), p. ex. x86_64-pc-windows-msvc
FIN
  exit 1
}

LOCAL=0
TRIPLE=""
for a in "$@"; do
  case "$a" in
    --local) LOCAL=1 ;;
    -h|--help) usage ;;
    -*) usage ;;
    *) TRIPLE="$a" ;;
  esac
done
[ -n "$TRIPLE" ] || TRIPLE="$(triple_hote)"

case "$TRIPLE" in
  *windows*) EXE=".exe" ;;
  *)         EXE="" ;;
esac
CIBLE="$DEST/typst-${TRIPLE}${EXE}"
mkdir -p "$DEST"

if [ "$LOCAL" = 1 ]; then
  SRC="$(command -v typst || true)"
  [ -n "$SRC" ] || { echo "aucun typst dans le PATH." >&2; exit 1; }
  VU="$("$SRC" --version | awk '{print $2}')"
  if [ "$VU" != "$VERSION" ]; then
    echo "typst du PATH en $VU, version épinglée $VERSION — refus." >&2
    echo "Relancer sans --local pour télécharger la bonne version." >&2
    exit 1
  fi
  cp "$SRC" "$CIBLE"
else
  BASE="https://github.com/typst/typst/releases/download/v${VERSION}"
  TMP="$(mktemp -d)"
  trap 'rm -rf "$TMP"' EXIT
  case "$TRIPLE" in
    *windows*)
      curl -fL "$BASE/typst-${TRIPLE}.zip" -o "$TMP/t.zip"
      # `unzip` n'est pas garanti dans Git Bash, où ce script tourne sur les runners
      # Windows ; le `tar` de Windows 10 et au-delà ouvre un zip.
      if command -v unzip >/dev/null 2>&1; then
        unzip -q "$TMP/t.zip" -d "$TMP"
      else
        tar -xf "$TMP/t.zip" -C "$TMP"
      fi
      ;;
    *)
      curl -fL "$BASE/typst-${TRIPLE}.tar.xz" -o "$TMP/t.tar.xz"
      tar -xJf "$TMP/t.tar.xz" -C "$TMP"
      ;;
  esac
  cp "$(find "$TMP" -name "typst${EXE}" -type f | head -1)" "$CIBLE"
fi

chmod +x "$CIBLE"
echo "$CIBLE — $("$CIBLE" --version 2>/dev/null || echo 'non exécutable sur cet hôte')"
