#!/usr/bin/env python3
"""Compose l'intérieur d'un roman prêt pour l'impression à la demande.

Usage : gen_interieur.py REPERTOIRE [--provider lulu] [-o SORTIE]
Exemple : outils/gen_interieur.py build/heures-creuses --provider lulu

REPERTOIRE est un répertoire de travail de build/ : il contient livre.toml, dont
les chemins (manuscrit, couverture) partent de build/ — « in/texts/roman.md »
désigne une ressource partagée, un chemin absolu est pris tel quel. Manuscrit :
titre en « # », chapitres en « ## NN - Titre ». Chaîne orchestrée : pandoc
(Markdown → HTML) → composition (liminaires, chapitres, gabarit @page du
prestataire) → weasyprint (HTML → PDF). Sorties dans REPERTOIRE/out/<provider>/ :
interieur-<provider>.pdf + les HTML intermédiaires (corps.html, interieur.html).
La gouttière dépend de la pagination : seconde passe automatique si le compte
de pages sort de la tranche supposée. Le nombre de pages final est affiché —
à reporter dans l'app (onglet Assemblage) pour le calcul du dos.
"""
import argparse
import html
import os
import re
import shutil
import subprocess
import sys
import zlib
from pathlib import Path

# tomllib exige Python >= 3.11 ; le python3 système peut être plus vieux :
# on se ré-exécute sur un interpréteur récent s'il en existe un.
if sys.version_info < (3, 11):
    for cand in ("python3.14", "python3.13", "python3.12", "python3.11"):
        exe = shutil.which(cand)
        if exe:
            os.execv(exe, [exe] + sys.argv)
    sys.exit("Python >= 3.11 requis (tomllib) ; aucun interpréteur récent trouvé.")

import tomllib

# Gabarits par prestataire — complète le PROVIDERS d'index.html (lui couvre la couverture, celui-ci l'intérieur).
# Gouttières par tranche de pagination : seules les tranches vérifiées dans le
# guide du prestataire figurent ici ; hors tranche, on refuse plutôt qu'inventer.
PROVIDERS = {
    "lulu": {
        "format": (108.0, 175.0),           # mm — Pocketbook
        "marge_haut": 14.0, "marge_bas": 15.0,
        "exterieur": 13.0,                  # mm — marge extérieure (sécurité)
        "gouttieres": [(151, 400, 25.0)],   # (pages min, pages max, marge intérieure mm) — guide Lulu
        "corps_pt": 9.5, "interligne": 1.42, "folio_pt": 8,
    },
}


def gouttiere(P, pages):
    for lo, hi, g in P["gouttieres"]:
        if lo <= pages <= hi:
            return g
    sys.exit(f"{pages} pages : tranche de gouttière absente du gabarit — la compléter "
             "dans PROVIDERS depuis le guide du prestataire.")


def pdf_pages(path):
    """Compte de pages : plus grand /Count du PDF, object streams zlib compris
    (les PDF weasyprint n'ont pas de /Count en clair)."""
    data = path.read_bytes()
    counts = [int(x) for x in re.findall(rb"/Count (\d+)", data)]
    for m in re.finditer(rb"stream\r?\n(.*?)\r?\nendstream", data, re.S):
        try:
            blob = zlib.decompress(m.group(1))
        except zlib.error:
            continue
        counts += [int(x) for x in re.findall(rb"/Count (\d+)", blob)]
    if not counts:
        sys.exit(f"Compte de pages introuvable dans {path}.")
    return max(counts)


def decoupe_chapitres(corps, attendu):
    corps = re.sub(r'<h1[^>]*>.*?</h1>\s*<p><em>[^<]*</em></p>\s*<hr />\s*', "", corps, count=1)
    corps = re.sub(r'\s*<hr />\s*', "\n", corps)
    parts = re.split(r'<h2[^>]*>(.*?)</h2>', corps)
    chapters = []
    for i in range(1, len(parts), 2):
        heading = parts[i].strip()
        body = parts[i + 1].strip()
        m = re.match(r'^(\d+)\s*(?:-\s*(.*))?$', heading)
        if not m:
            sys.exit(f"Titre de chapitre inattendu : « {heading} » (attendu : « NN - Titre »).")
        chapters.append((int(m.group(1)), (m.group(2) or "").strip(), body))
    # « chapitres » est un contrôle d'intégrité facultatif : il n'a de sens qu'au
    # gel, quand le compte ne doit plus bouger. Absent, on compose ce qu'on trouve.
    if attendu is not None and len(chapters) != attendu:
        sys.exit(f"{attendu} chapitres attendus (livre.toml), {len(chapters)} trouvés.")
    return chapters


def compose(livre, P, gut, chapters):
    e = html.escape
    fw, fh = P["format"]
    ext = P["exterieur"]
    titre_page = e(livre.get("titre_page", livre["titre"])).replace("\n", "<br>")
    copyright_html = e(livre["copyright"]).replace("\n", "<br>\n  ")
    sections = []
    for num, title, body in chapters:
        # title et body sortent de pandoc : déjà en HTML (échappés) — ne pas ré-échapper
        title_html = f'<p class="chapitre-titre">{title}</p>' if title else ""
        sections.append(
            f'<section class="chapitre">\n'
            f'<header class="chapitre-tete"><p class="chapitre-numero">{num}</p>{title_html}</header>\n'
            f'{body}\n</section>'
        )
    corps_html = "\n".join(sections)
    return f"""<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>{e(livre["titre"])}</title>
<style>
@page {{
  size: {fw}mm {fh}mm;
  margin-top: {P["marge_haut"]}mm;
  margin-bottom: {P["marge_bas"]}mm;
  @bottom-center {{
    content: counter(page);
    font-family: Baskerville, serif;
    font-size: {P["folio_pt"]}pt;
    color: #000;
  }}
}}
/* Gouttière selon la tranche de pagination du guide prestataire. Page 1 = recto (droite). */
@page :right {{ margin-left: {gut}mm; margin-right: {ext}mm; }}
@page :left  {{ margin-left: {ext}mm; margin-right: {gut}mm; }}
/* Pages liminaires sans folio. */
@page liminaire {{
  @bottom-center {{ content: none; }}
}}
html {{ font-family: Baskerville, serif; font-size: {P["corps_pt"]}pt; line-height: {P["interligne"]}; }}
body {{ margin: 0; color: #000; }}
p {{
  margin: 0;
  text-indent: 1.2em;
  text-align: justify;
  hyphens: auto;
  orphans: 2;
  widows: 2;
}}
em {{ font-style: italic; }}

/* — Pages liminaires — */
.liminaire {{ page: liminaire; page-break-after: always; }}
.faux-titre {{ text-align: center; }}
.faux-titre p {{ text-indent: 0; text-align: center; margin-top: 42mm;
  font-size: 11pt; letter-spacing: 0.12em; text-transform: uppercase; }}
.page-blanche {{ min-height: 1mm; }}
.page-titre {{ text-align: center; }}
.page-titre .auteur {{ margin-top: 30mm; font-size: 10.5pt; letter-spacing: 0.1em;
  text-transform: uppercase; text-indent: 0; text-align: center; }}
.page-titre .titre {{ margin-top: 14mm; font-size: 15pt; letter-spacing: 0.06em;
  text-transform: uppercase; text-indent: 0; text-align: center; line-height: 1.3; }}
.page-titre .genre {{ margin-top: 10mm; font-style: italic; font-size: 10pt;
  text-indent: 0; text-align: center; }}
.page-copyright {{ display: flex; flex-direction: column; justify-content: flex-end;
  height: 143mm; }}
.page-copyright p {{ text-indent: 0; text-align: center; font-size: 8pt; line-height: 1.5; }}

/* — Chapitres — */
section.chapitre {{ page-break-before: always; }}
.chapitre-tete {{ margin-top: 22mm; margin-bottom: 11mm; }}
.chapitre-numero {{ text-indent: 0; text-align: center; font-size: 13pt;
  margin-bottom: 3.5mm; }}
.chapitre-titre {{ text-indent: 0; text-align: center; font-size: 10pt;
  letter-spacing: 0.14em; text-transform: uppercase; line-height: 1.5; }}
.chapitre-tete + p, .chapitre-tete ~ p:first-of-type {{ text-indent: 0; }}
section.chapitre > header + p {{ text-indent: 0; }}
</style>
</head>
<body>

<div class="liminaire faux-titre"><p>{e(livre["titre"])}</p></div>
<div class="liminaire page-blanche"></div>
<div class="liminaire page-titre">
  <p class="auteur">{e(livre["auteur"])}</p>
  <p class="titre">{titre_page}</p>
  <p class="genre">{e(livre.get("genre", "roman"))}</p>
</div>
<div class="liminaire page-copyright">
  <p>{copyright_html}</p>
</div>

{corps_html}

</body>
</html>
"""


def main():
    p = argparse.ArgumentParser(description="Compose l'intérieur d'un roman (pandoc → weasyprint).")
    p.add_argument('repertoire', type=Path, help='répertoire du roman (contient livre.toml)')
    p.add_argument('--provider', default='lulu', choices=sorted(PROVIDERS),
                   help='gabarit prestataire (défaut : lulu)')
    p.add_argument('-o', '--sortie', type=Path, default=None,
                   help='PDF de sortie (défaut : REPERTOIRE/<provider>/interieur-<provider>.pdf)')
    args = p.parse_args()

    for outil in ('pandoc', 'weasyprint'):
        if not shutil.which(outil):
            sys.exit(f"{outil} introuvable dans le PATH (brew install {outil}).")

    repertoire = args.repertoire.resolve()
    toml_path = repertoire / 'livre.toml'
    if not toml_path.is_file():
        sys.exit(f"Fichier introuvable : {toml_path}")
    livre = tomllib.loads(toml_path.read_text(encoding='utf-8'))['livre']

    # Les chemins du livre.toml partent de build/, le parent du répertoire de
    # travail : « in/texts/roman.md » désigne la ressource partagée. Un chemin
    # absolu est pris tel quel (pathlib ignore alors la racine).
    racine = repertoire.parent
    manuscrit = racine / livre.get('manuscrit', 'text.md')
    if not manuscrit.is_file():
        sys.exit(f"Manuscrit introuvable : {manuscrit}")

    P = PROVIDERS[args.provider]
    outdir = repertoire / 'out' / args.provider
    outdir.mkdir(parents=True, exist_ok=True)
    pdf = args.sortie or outdir / f'interieur-{args.provider}.pdf'

    corps_path = outdir / 'corps.html'
    subprocess.run(['pandoc', str(manuscrit),
                    '-f', 'markdown-yaml_metadata_block-simple_tables-multiline_tables-pipe_tables-grid_tables',
                    '-t', 'html', '--wrap=none', '-o', str(corps_path)], check=True)
    chapters = decoupe_chapitres(corps_path.read_text(encoding='utf-8'), livre.get('chapitres'))

    gut = P["gouttieres"][0][2]   # hypothèse de départ : première tranche du gabarit
    for _ in range(2):
        interieur = outdir / 'interieur.html'
        interieur.write_text(compose(livre, P, gut, chapters), encoding='utf-8')
        subprocess.run(['weasyprint', str(interieur), str(pdf)], check=True)
        pages = pdf_pages(pdf)
        g2 = gouttiere(P, pages)  # sort proprement si la tranche est inconnue
        if g2 == gut:
            break
        gut = g2                  # la gouttière change la pagination : re-composer
    else:
        sys.exit("La gouttière ne converge pas (pagination oscillante entre deux tranches).")

    print(f"{pdf} — {pages} pages, {len(chapters)} chapitres, gouttière {gut} mm "
          f"({args.provider}). Reporte {pages} pages dans l'app (onglet Assemblage) pour le dos.")


if __name__ == '__main__':
    main()
