#!/usr/bin/env python3
"""Génère le PDF d'un roman au format poche depuis un répertoire de travail.

Usage : roman_pdf.py REPERTOIRE NB_CHAPITRES [-t TAILLE] [-o SORTIE]
Exemple : roman_pdf.py build/mon-roman 8 -t 10

REPERTOIRE est un répertoire de travail de build/ : il contient livre.toml, dont
les chemins (manuscrit, couverture) partent de build/ — « in/texts/roman.md »
désigne une ressource partagée, un chemin absolu est pris tel quel. Manuscrit :
titre en «# », chapitres en «## », séparateurs de scène «---», italiques *…*,
gras **…**. La couverture est posée en pleine page. Pages numérotées en bas à
droite, couverture exclue. Sortie : REPERTOIRE/out/roman.pdf.
"""

import argparse
import re
import sys
from pathlib import Path

from fpdf import FPDF
from fpdf.enums import XPos, YPos

# Pas de ré-exécution sur un interpréteur récent comme dans gen_interieur.py :
# fpdf2 n'est installé que sur le python3 système, plus ancien que tomllib.
# tomli est le backport du même parseur, à l'API identique.
try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib

# gabarit poche (Folio : 108 x 178 mm), Times New Roman
PAGE = (108, 178)
MARGE_G, MARGE_D, MARGE_H, MARGE_B = 15, 15, 16, 18
INTERLIGNE = 1.35
FONTES = {
    '':   '/System/Library/Fonts/Supplemental/Times New Roman.ttf',
    'B':  '/System/Library/Fonts/Supplemental/Times New Roman Bold.ttf',
    'I':  '/System/Library/Fonts/Supplemental/Times New Roman Italic.ttf',
    'BI': '/System/Library/Fonts/Supplemental/Times New Roman Bold Italic.ttf',
}


class RomanPDF(FPDF):
    def footer(self):
        if self.page_no() == 1:          # pas de folio sur la couverture
            return
        self.set_y(-13)
        self.set_font('Roman', '', 9)
        self.cell(0, 5, str(self.page_no()), align='R')


def emphases_fpdf(texte):
    """Markdown standard → syntaxe fpdf2 : *italique* → __italique__."""
    texte = re.sub(r'\*\*(.+?)\*\*', '\x00\\1\x00', texte)   # gras en réserve
    texte = re.sub(r'(?<!\w)[*_](.+?)[*_](?!\w)', r'__\1__', texte)
    return texte.replace('\x00', '**')


def decoupe_chapitres(md):
    """Retourne (titre du livre, [(titre de chapitre, [blocs]), …]).
    Un bloc est un paragraphe ou le séparateur de scène '---'."""
    titre = ''
    chapitres = []
    courant = None
    for ligne in md.splitlines():
        l = ligne.strip()
        if l.startswith('## '):
            courant = (l[3:].strip(), [])
            chapitres.append(courant)
        elif l.startswith('# ') and not titre:
            titre = l[2:].strip()
        elif courant is not None and (l == '---' or l):
            courant[1].append('---' if l == '---' else l)
    return titre, chapitres


def main():
    p = argparse.ArgumentParser(
        description="Génère le PDF d'un roman au format poche (couverture + chapitres).")
    p.add_argument('repertoire', type=Path, help='répertoire du roman (contient livre.toml)')
    p.add_argument('chapitres', type=int, help='nombre de chapitres à inclure')
    p.add_argument('-t', '--taille', type=float, default=10,
                   help='taille de police du texte courant, en points (défaut : 10)')
    p.add_argument('-o', '--sortie', type=Path, default=None,
                   help='fichier PDF de sortie (défaut : roman.pdf dans le répertoire)')
    args = p.parse_args()

    repertoire = args.repertoire.resolve()
    toml_path = repertoire / 'livre.toml'
    if not toml_path.is_file():
        sys.exit(f"Fichier introuvable : {toml_path}")
    livre = tomllib.loads(toml_path.read_text(encoding='utf-8'))['livre']

    # Les chemins du livre.toml partent de build/, le parent du répertoire de
    # travail : « in/covers/face.png » désigne la ressource partagée. Un chemin
    # absolu est pris tel quel (pathlib ignore alors la racine).
    racine = repertoire.parent
    manuscrit = racine / livre.get('manuscrit', 'text.md')
    couverture = racine / livre.get('couverture', 'cover.png')
    for f in (manuscrit, couverture):
        if not f.is_file():
            sys.exit(f"Fichier introuvable : {f}")

    titre, chapitres = decoupe_chapitres(manuscrit.read_text(encoding='utf-8'))
    if args.chapitres < 1 or args.chapitres > len(chapitres):
        sys.exit(f"{args.chapitres} chapitres demandés, {len(chapitres)} disponibles.")
    chapitres = chapitres[:args.chapitres]

    pdf = RomanPDF(unit='mm', format=PAGE)
    pdf.add_font('Roman', '', FONTES[''])
    pdf.add_font('Roman', 'B', FONTES['B'])
    pdf.add_font('Roman', 'I', FONTES['I'])
    pdf.add_font('Roman', 'BI', FONTES['BI'])
    if titre:
        pdf.set_title(titre)
    pdf.set_margins(MARGE_G, MARGE_H, MARGE_D)
    pdf.set_auto_page_break(True, margin=MARGE_B)

    # première de couverture, pleine page
    pdf.add_page()
    pdf.image(str(couverture), x=0, y=0, w=PAGE[0], h=PAGE[1])

    h_ligne = args.taille * INTERLIGNE * 0.3528   # points → mm
    for titre_ch, blocs in chapitres:
        pdf.add_page()
        pdf.set_y(MARGE_H + 24)
        pdf.set_font('Roman', 'B', args.taille + 1.5)
        pdf.multi_cell(0, h_ligne * 1.2, titre_ch, align='C',
                       new_x=XPos.LMARGIN, new_y=YPos.NEXT)
        pdf.ln(h_ligne * 2)
        pdf.set_font('Roman', '', args.taille)
        premier = True
        for bloc in blocs:
            if bloc == '---':
                pdf.ln(h_ligne)     # blanc de scène
                premier = True
                continue
            alinea = '' if premier else '\u2003'   # alinéa d'un cadratin
            # new_x : sinon fpdf2 laisse le curseur au bord droit et le
            # paragraphe suivant n'a plus de largeur disponible
            pdf.multi_cell(0, h_ligne, alinea + emphases_fpdf(bloc),
                           align='J', markdown=True,
                           new_x=XPos.LMARGIN, new_y=YPos.NEXT)
            premier = False

    # L'épreuve de lecture ne vise aucun éditeur : elle sort dans out/, à côté
    # des out/<éditeur>/ qui reçoivent les packages.
    sortie = args.sortie or repertoire / 'out' / 'roman.pdf'
    sortie.parent.mkdir(parents=True, exist_ok=True)
    pdf.output(str(sortie))
    print(f"{sortie} — {pdf.page_no()} pages, {len(chapitres)} chapitres, "
          f"Times New Roman {args.taille} pt, {PAGE[0]}x{PAGE[1]} mm")


if __name__ == '__main__':
    main()
