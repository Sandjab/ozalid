#!/usr/bin/env python3
"""Assemble les images d'un répertoire en une planche PNG sans redimensionnement.

Usage : planche.py REPERTOIRE COLONNESxLIGNES [-o SORTIE]
Exemple : planche.py images/mosaic 3x2

Les images (png/jpg/jpeg/webp, ordre alphabétique) sont posées à leur taille
native, centrées dans des cellules calées sur la plus grande d'entre elles —
aucun rééchantillonnage, donc aucune perte de qualité.
"""

import argparse
import re
import sys
from pathlib import Path

from PIL import Image

FOND = (255, 255, 255)   # fond et gouttières : blanc
MARGE = 24               # gouttière entre cellules et bord, en px

EXTENSIONS = {'.png', '.jpg', '.jpeg', '.webp'}


def main():
    p = argparse.ArgumentParser(
        description="Assemble les images d'un répertoire en une planche PNG sans redimensionnement.")
    p.add_argument('repertoire', type=Path, help='répertoire contenant les images')
    p.add_argument('grille', help='dimension de la planche, ex. 3x2 (colonnes x lignes)')
    p.add_argument('-o', '--sortie', type=Path, default=None,
                   help='fichier PNG de sortie (défaut : planche-CxL.png dans le répertoire)')
    args = p.parse_args()

    m = re.fullmatch(r'(\d+)\s*[xX×]\s*(\d+)', args.grille)
    if not m:
        sys.exit(f"Grille invalide : « {args.grille} » (attendu : COLONNESxLIGNES, ex. 3x2)")
    cols, rows = int(m.group(1)), int(m.group(2))
    if cols * rows == 0:
        sys.exit("La grille doit avoir au moins une cellule.")

    if not args.repertoire.is_dir():
        sys.exit(f"Répertoire introuvable : {args.repertoire}")
    sortie = args.sortie or args.repertoire / f'planche-{cols}x{rows}.png'
    fichiers = sorted(f for f in args.repertoire.iterdir()
                      if f.suffix.lower() in EXTENSIONS and not f.name.startswith('.')
                      and f.resolve() != sortie.resolve())
    if not fichiers:
        sys.exit(f"Aucune image ({'/'.join(sorted(EXTENSIONS))}) dans {args.repertoire}")
    if len(fichiers) > cols * rows:
        sys.exit(f"{len(fichiers)} images pour {cols * rows} cellules ({cols}x{rows}) : "
                 "agrandis la grille ou retire des images.")
    if len(fichiers) < cols * rows:
        print(f"Attention : {len(fichiers)} images pour {cols * rows} cellules, "
              "les dernières resteront vides.", file=sys.stderr)

    images = [Image.open(f) for f in fichiers]
    cw = max(im.width for im in images)
    ch = max(im.height for im in images)

    planche = Image.new('RGB', (MARGE + cols * (cw + MARGE), MARGE + rows * (ch + MARGE)), FOND)
    for i, im in enumerate(images):
        cx = MARGE + (i % cols) * (cw + MARGE) + (cw - im.width) // 2
        cy = MARGE + (i // cols) * (ch + MARGE) + (ch - im.height) // 2
        planche.paste(im, (cx, cy), im if im.mode == 'RGBA' else None)

    planche.save(sortie, 'PNG')
    print(f"{sortie} — {planche.width}x{planche.height} px, "
          f"{len(images)} images en cellules de {cw}x{ch}")


if __name__ == '__main__':
    main()
