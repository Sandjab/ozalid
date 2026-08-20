#!/usr/bin/env bash
# Récupère les polices de couverture dans src-tauri/fonts/.
#
# Typst ne lit que des fichiers : contrairement au navigateur, il ne va pas chercher
# une police chez Google. Les embarquer est donc la condition pour que la même
# maquette rende pareil sur macOS et sur Windows — la raison d'être du choix Typst.
#
# Toutes sont sous licence OFL, redistribuables avec l'application. Les fichiers
# entre crochets sont des polices variables : un seul fichier couvre toutes les
# graisses, et Typst sait interpoler l'axe wght.
#
# Georgia et Helvetica, proposées par l'atelier HTML, ne sont PAS reprises : elles
# appartiennent au système, ne sont pas redistribuables, et Helvetica n'existe pas
# sous Windows. Une maquette qui les utilise est signalée à l'import.
set -euo pipefail

BASE="https://raw.githubusercontent.com/google/fonts/main/ofl"
ICI="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ICI/src-tauri/fonts"

# répertoire ofl → fichiers à prendre
FICHIERS=(
  "bodonimoda/BodoniModa[opsz,wght].ttf"
  "bodonimoda/BodoniModa-Italic[opsz,wght].ttf"
  "playfairdisplay/PlayfairDisplay[wght].ttf"
  "playfairdisplay/PlayfairDisplay-Italic[wght].ttf"
  "prata/Prata-Regular.ttf"
  "spectral/Spectral-Regular.ttf"
  "spectral/Spectral-Italic.ttf"
  "spectral/Spectral-SemiBold.ttf"
  "spectral/Spectral-SemiBoldItalic.ttf"
  "spectral/Spectral-Bold.ttf"
  "spectral/Spectral-BoldItalic.ttf"
  "ebgaramond/EBGaramond[wght].ttf"
  "ebgaramond/EBGaramond-Italic[wght].ttf"
  "librebaskerville/LibreBaskerville[wght].ttf"
  "librebaskerville/LibreBaskerville-Italic[wght].ttf"
  "archivo/Archivo[wdth,wght].ttf"
  "archivo/Archivo-Italic[wdth,wght].ttf"
  "librefranklin/LibreFranklin[wght].ttf"
  "librefranklin/LibreFranklin-Italic[wght].ttf"
  "oswald/Oswald[wght].ttf"
)

mkdir -p "$DEST"
for f in "${FICHIERS[@]}"; do
  nom="${f##*/}"
  cible="$DEST/$nom"
  [ -f "$cible" ] && continue
  # --data-urlencode ferait un POST ; on encode seulement les crochets du nom.
  url="$BASE/${f//\[/%5B}"
  url="${url//\]/%5D}"
  url="${url//,/%2C}"
  curl -fsSL "$url" -o "$cible"
  echo "  $nom"
done

echo "$DEST — $(find "$DEST" -name '*.ttf' | wc -l | tr -d ' ') fichiers, $(du -sh "$DEST" | cut -f1)"
