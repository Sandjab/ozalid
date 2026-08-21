#!/usr/bin/env bash
# Récupère les polices de l'application dans src-tauri/fonts/ — couverture et
# intérieur.
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
  # Polices de labeur de l'intérieur. Cardo n'a pas de version variable : ses trois
  # coupes sont des fichiers statiques.
  "crimsonpro/CrimsonPro[wght].ttf"
  "crimsonpro/CrimsonPro-Italic[wght].ttf"
  "alegreya/Alegreya[wght].ttf"
  "alegreya/Alegreya-Italic[wght].ttf"
  "vollkorn/Vollkorn[wght].ttf"
  "vollkorn/Vollkorn-Italic[wght].ttf"
  "cardo/Cardo-Regular.ttf"
  "cardo/Cardo-Italic.ttf"
  "cardo/Cardo-Bold.ttf"
  # Mains manuscrites des envois autographes. Retenues sur relevé fontTools, pas sur
  # la fiche du fondeur : chacune porte les accents français, la ligature œ, les
  # guillemets et l'apostrophe courbe. Une main qui les ignorerait serait composée par
  # repli, sans un mot, et l'envoi partirait chez le dédicataire dans deux écritures —
  # le mécanisme même contre lequel `Envois::verifie` est posé.
  "caveat/Caveat[wght].ttf"
  "dancingscript/DancingScript[wght].ttf"
  "petitformalscript/PetitFormalScript-Regular.ttf"
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
  # La sortie passe par un nom neutre, puis le shell renomme. Sous Git Bash — où ce
  # script tourne sur les runners Windows —, un `-o` dont le nom porte des crochets et
  # une virgule fait échouer curl à l'écriture (« curl: (23) »), et toutes les polices
  # variables en portent. Le renommage, lui, ne passe pas par curl.
  curl -fsSL "$url" -o "$DEST/.police-en-cours"
  mv "$DEST/.police-en-cours" "$cible"
  echo "  $nom"
done

echo "$DEST — $(find "$DEST" -name '*.ttf' | wc -l | tr -d ' ') fichiers, $(du -sh "$DEST" | cut -f1)"
