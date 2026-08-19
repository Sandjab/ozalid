# Lot 5 — Outils Python, réorganisation de build/ et README : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** tracker la chaîne Python de composition de l'intérieur (paramétrée multi-romans × multi-prestataires) dans `outils/`, réorganiser `build/` par roman (toujours non tracké), purger trois micro-dettes d'app du lot 4, et réécrire le README (app à jour + modops des outils + organisation du dépôt).

**Architecture :** `outils/` (tracké) reçoit `roman_pdf.py` et `planche.py` (déplacés tels quels) et un `gen_interieur.py` réécrit : lit `build/<roman>/livre.toml`, gabarits par prestataire dans un dict `PROVIDERS` (miroir de l'app), orchestre pandoc → composition → weasyprint (CLI, subprocess), compte les pages du PDF produit (scan `/Count`, streams zlib décompressés), seconde passe automatique si la pagination sort de la tranche de gouttière supposée. `build/<roman>/` = manuscrit + `cover.png` + `livre.toml` + un sous-répertoire de sorties par usage. La couverture n'est plus dans la chaîne Python (l'app l'exporte — lot 3) ; `gen_couverture.py` reste en fin de vie dans `build/`, non migré.

**Environnement relevé (2026-08-19, cette machine) :** `python3` = 3.9.6 (PAS de `tomllib`) ; `python3.12` et `python3.14` présents via brew → le script se ré-exécute sur un Python ≥ 3.11 s'il démarre sur plus vieux. `pandoc` et `weasyprint` (CLI 68.1) dans le PATH ; l'API Python de weasyprint n'est PAS importable depuis le python système → orchestration par subprocess. `fpdf2` et `PIL` disponibles (roman_pdf/planche fonctionnent). `mdls` donne le compte de pages mais dépend de Spotlight → le script embarque son propre compteur (le `/Count` des PDF weasyprint est dans des object streams compressés — vérifié : `grep -a "/Count"` ne trouve rien en clair).

**Spécification :** `docs/superpowers/specs/2026-08-19-image-quatrieme-et-chaine-design.md` (§3-5).
**Inclus en plus de la spec** (dettes consignées à la clôture du lot 4) : note de dégradation typo du prolongement ; `applyConfig` remet la photo de la 1ère au défaut quand `c.image` est absent (symétrie avec `image4`) ; la note d'export PNG mentionne la photo quand seule `image4` est embarquée.
**Hors périmètre :** Amazon KDP ; tranches de gouttière Lulu au-delà de 151-400 pages (le script refuse proprement au lieu d'inventer — à compléter depuis le guide quand le besoin viendra).

**Vérification :** pour la tâche 1 (app) : `node --check` + sondes navigateur comme aux lots précédents (serve.sh, jamais `python3 -m http.server`). Pour la chaîne : exécutions réelles, non-régression forte = régénérer l'intérieur du livre réel → **244 pages** et `interieur.html` recomposé équivalent à l'ancien `build/lulu/src/interieur.html` (diff textuel : seules différences attendues, aucune ou espaces).

**Extraction JS (tâche 1)** :

```bash
node -e "
const html = require('fs').readFileSync('index.html','utf8');
const blocks = [...html.matchAll(/<script(?![^>]*src)[^>]*>([\s\S]*?)<\/script>/g)].map(m=>m[1]);
require('fs').writeFileSync('/tmp/ozalid-extrait.js', blocks.join('\n;\n'));
" && node --check /tmp/ozalid-extrait.js && echo "Syntaxe OK"
```

**Repères :** `index.html` : **ne jamais le lire en entier** (ligne base64 ~90 Ko) ; ancres : `noteQ4Pro`/`noteQ4NoImg`/`noteQ4Manque` (fieldset fsQ4Image), bloc 4ème de `render()` (`q4m`, `proOn`, `pano`), `applyConfig` (`c.image`/`c.image4`), `DEFAULT_IMG`, écouteur `btnPng` (variable `note`). Racine : `roman_pdf.py`, `planche.py` (trackés). Non tracké : `build/lulu/{text.md,cover.png,src/{gen_interieur.py,gen_couverture.py,corps.html,interieur.html},interieur-poche.pdf,couverture-poche.pdf,LISEZMOI.md,lulu-book-creation-guide.pdf}`, `build/WIP.md`, `build/rox/{WIP.md,LHC-Photo.png,cover.png→lien,roman.pdf}`, `build/delf/{cover.png,roman.pdf}`. Données du livre réel (source : l'ancien `gen_interieur.py` et `LISEZMOI.md`) : titre « Les Heures creuses », auteur « Ivan Pjig », genre « roman », copyright 3 lignes (« © Ivan Pjig, 2026. / Tous droits réservés. / Maquette de couverture : atelier Ozalid. »), titre de page de titre coupé « Les Heures / creuses », 55 chapitres, manuscrit `text.md`, 244 pages.

---

### Tâche 1 : micro-dettes d'app du lot 4

**Files :** Modify `index.html`

- [x] **Step 1 : note de dégradation typo du prolongement**

Dans le fieldset `fsQ4Image`, après la ligne de `noteQ4Pro`, ajouter :

```html
        <p class="note hide" id="noteQ4Typo">La 1ère n'a pas d'image (mode sans image) : le prolongement affiche le papier.</p>
```

Dans le bloc 4ème de `render()`, ajouter la visibilité (près des autres notes) :

```js
  $('noteQ4Typo').classList.toggle('hide', q4m !== 'prolongement' || proOn);
```

(la note apparaît quand le prolongement est demandé mais impossible — mode typo ou image non chargée ; place la ligne APRÈS le calcul de `proOn`).

- [x] **Step 2 : `applyConfig` symétrique pour la photo de la 1ère**

La ligne `if (c.image) $('elImg').src = c.image;` devient :

```js
  if (c.image) $('elImg').src = c.image; else $('elImg').src = DEFAULT_IMG;
```

ATTENTION : `DEFAULT_IMG` est déclaré plus bas dans le script (`const DEFAULT_IMG = $('elImg').src;` au démarrage) mais `applyConfig` n'est jamais appelée avant cette initialisation — vérifie-le (grep des appels d'`applyConfig` : tous dans des écouteurs ou après `loadLocal()`). Si un chemin l'appelait avant, STOP et rapporte.

- [x] **Step 3 : note d'export PNG**

Dans l'écouteur `btnPng`, la ligne qui construit `note` (`note = ' — réglages' + (cfg.image ? ' et photo source embarqués' : ' embarqués');`) devient :

```js
      note = ' — réglages' + (cfg.image || cfg.image4 ? ' et photo source embarqués' : ' embarqués');
```

- [x] **Step 4 : syntaxe + sondes + commit**

`node --check` (en-tête). Sondes (session relevée/restaurée, serveur stoppé) : (1) preset Blanche (typo) + `inQ4BgMode='prolongement'` → `noteQ4Typo` visible, `noteQ4Pro` visible, rien de cassé ; preset Surimpression + prolongement avec matière → `noteQ4Typo` cachée ; (2) `applyConfig` d'une config sans `image` → `$('elImg').src === DEFAULT_IMG` (et l'aperçu montre l'image par défaut) ; avec `image` → restaurée ; (3) round-trip complet inchangé (0 divergence). Commit :

```bash
git add index.html
git commit -m "4ème : note typo du prolongement ; photo 1ère au défaut si absente ; note d'export"
```

---

### Tâche 2 : `outils/` — déplacer les utilitaires existants

**Files :** Create `outils/` ; Move `roman_pdf.py`, `planche.py`

- [x] **Step 1 : déplacements**

```bash
mkdir -p outils
git mv roman_pdf.py outils/roman_pdf.py
git mv planche.py outils/planche.py
```

Aucune modification de contenu (les deux scripts sont autonomes, sans chemin relatif au dépôt).

- [x] **Step 2 : vérification d'exécution**

```bash
python3 outils/roman_pdf.py --help && python3 outils/planche.py --help
```

Attendu : les deux usages s'affichent sans erreur. Vérifier aussi `git status` : renames détectés, rien d'autre.

- [x] **Step 3 : commit**

```bash
git commit -m "Outils : roman_pdf et planche déménagent dans outils/"
```

---

### Tâche 3 : `outils/gen_interieur.py` paramétré

**Files :** Create `outils/gen_interieur.py`

- [x] **Step 1 : le script**

Créer `outils/gen_interieur.py` (exécutable, `chmod +x`) avec ce contenu — le gabarit HTML/CSS reprend À L'IDENTIQUE celui de `build/lulu/src/gen_interieur.py` (lis-le d'abord ; seuls le `@page` et les textes des liminaires deviennent paramétriques) :

```python
#!/usr/bin/env python3
"""Compose l'intérieur d'un roman prêt pour l'impression à la demande.

Usage : gen_interieur.py REPERTOIRE [--provider lulu] [-o SORTIE]
Exemple : outils/gen_interieur.py build/heures-creuses --provider lulu

REPERTOIRE contient livre.toml (métadonnées) et le manuscrit Markdown qu'il
désigne (titre en « # », chapitres en « ## NN - Titre »). Chaîne orchestrée :
pandoc (Markdown → HTML) → composition (liminaires, chapitres, gabarit @page
du prestataire) → weasyprint (HTML → PDF). Sorties dans REPERTOIRE/<provider>/ :
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

# Gabarits par prestataire — miroir de PROVIDERS dans index.html.
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
    if len(chapters) != attendu:
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

    toml_path = args.repertoire / 'livre.toml'
    if not toml_path.is_file():
        sys.exit(f"Fichier introuvable : {toml_path}")
    livre = tomllib.loads(toml_path.read_text(encoding='utf-8'))['livre']

    manuscrit = args.repertoire / livre.get('manuscrit', 'text.md')
    if not manuscrit.is_file():
        sys.exit(f"Manuscrit introuvable : {manuscrit}")

    P = PROVIDERS[args.provider]
    outdir = args.repertoire / args.provider
    outdir.mkdir(exist_ok=True)
    pdf = args.sortie or outdir / f'interieur-{args.provider}.pdf'

    corps_path = outdir / 'corps.html'
    subprocess.run(['pandoc', str(manuscrit), '-f', 'markdown', '-t', 'html',
                    '-o', str(corps_path)], check=True)
    chapters = decoupe_chapitres(corps_path.read_text(encoding='utf-8'), livre['chapitres'])

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

    print(f"{pdf} — {pages} pages, gouttière {gut} mm ({args.provider}). "
          f"Reporte {pages} pages dans l'app (onglet Assemblage) pour le dos.")


if __name__ == '__main__':
    main()
```

- [x] **Step 2 : vérifications à froid**

```bash
chmod +x outils/gen_interieur.py
python3 outils/gen_interieur.py --help
python3 -m py_compile outils/gen_interieur.py && echo "compile OK"
outils/gen_interieur.py /tmp/inexistant 2>&1 | tail -1
```

Attendu : usage affiché (via la ré-exécution 3.12+ transparente) ; compilation OK ; erreur propre « Fichier introuvable : /tmp/inexistant/livre.toml ».

- [x] **Step 3 : commit**

```bash
git add outils/gen_interieur.py
git commit -m "Outils : gen_interieur paramétré (livre.toml, presets prestataire, orchestration)"
```

---

### Tâche 4 : réorganisation locale de `build/` + `livre.toml`

**Files :** aucun fichier tracké (tout est sous `build/`, gitignoré) — documenter chaque déplacement dans le rapport.

- [x] **Step 1 : arborescence par roman**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid/build
mkdir -p heures-creuses
mv lulu heures-creuses/lulu
mv heures-creuses/lulu/text.md heures-creuses/text.md
mv heures-creuses/lulu/cover.png heures-creuses/cover.png
mv WIP.md heures-creuses/WIP.md
mv rox heures-creuses/rox
mv delf heures-creuses/delf
```

(`gen_couverture.py` et l'ancien `src/` restent dans `heures-creuses/lulu/src/` — fin de vie, non migré. `rox/WIP.md` est un doublon du WIP racine : le laisser tel quel dans rox/, c'est de l'archive.)

- [x] **Step 2 : `livre.toml`**

Créer `build/heures-creuses/livre.toml` :

```toml
[livre]
titre = "Les Heures creuses"
titre_page = "Les Heures\ncreuses"
auteur = "Ivan Pjig"
genre = "roman"
copyright = """© Ivan Pjig, 2026.
Tous droits réservés.
Maquette de couverture : atelier Ozalid."""
chapitres = 55
manuscrit = "text.md"
```

- [x] **Step 3 : vérifications**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
git status --short          # rien : build/ intégralement ignoré
git check-ignore -v build/heures-creuses/livre.toml
ls build/heures-creuses build/heures-creuses/lulu
```

Attendu : `git status` vide ; `check-ignore` matche la règle `build/` ; l'arborescence est en place. Pas de commit (rien de tracké) — le rapport liste les déplacements.

---

### Tâche 5 : non-régression de la chaîne — régénérer l'intérieur du livre réel

**Files :** aucun (exécution + comparaison)

- [x] **Step 1 : régénération**

```bash
cd /Users/jean-paulgavini/Documents/Dev/ozalid
outils/gen_interieur.py build/heures-creuses --provider lulu
```

Attendu : sortie « … interieur-lulu.pdf — **244 pages**, gouttière 25 mm (lulu) … » sans erreur, en une passe (244 ∈ 151-400).

- [x] **Step 2 : équivalence de composition**

```bash
diff build/heures-creuses/lulu/src/interieur.html build/heures-creuses/lulu/interieur.html
```

Attendu : aucune différence de CONTENU. Différences attendues et tolérées, à lister une à une : formatage numérique du bloc `<style>` (f-strings Python : `108.0mm` vs `108mm`, `25.0mm` vs `25mm`…) et éventuels échappements `html.escape` sur les seuls champs du TOML (titre/auteur/copyright — pas sur le corps ni les titres de chapitres, qui sortent de pandoc déjà échappés). Toute différence dans le corps du texte ou la structure des sections est un échec. Comparer aussi le compte de pages avec l'ancien PDF : `mdls -raw -name kMDItemNumberOfPages build/heures-creuses/lulu/interieur-poche.pdf` → 244 = 244.

- [x] **Step 3 : contrôle visuel d'échantillon**

Rendre la première page des deux PDF (`sips -s format png <pdf> --out <png>` rend la page 1) et les comparer visuellement (faux-titre identique). Si un outil de rendu multi-pages est disponible (`pdftoppm`), comparer aussi une page de chapitre ; sinon le dire dans le rapport (page 1 seule vérifiée visuellement, le diff HTML couvrant le reste).

- [x] **Step 4 : consigner** les résultats au rapport (pas de commit — tout est sous `build/`).

---

### Tâche 6 : README réécrit

**Files :** Modify `README.md`

- [x] **Step 1 : réécriture**

Réécrire `README.md` en gardant le ton et le format existants (lis-le d'abord). Structure imposée et points obligatoires :

1. **# Ozalid** — description élargie : atelier de packaging de couverture pour l'auto-édition (maquette de 1ère, 4ème, planche complète, export prestataire) ; le nom (ozalid = épreuve de contrôle) reste expliqué.
2. **Usage** — ouvrir `index.html`, rien à installer (les outils Python sont optionnels, voir leur section).
3. **Ce que ça fait** — réécrit : les trois onglets (1ère : les trois modes existants + cadre Gallimard ; 4ème : texte, pied, zone ISBN, fond `papier | couleur | image propre | prolongement panoramique de la 1ère` ; Assemblage : prestataire Lulu, dos calculé du nombre de pages, planche 4ème|dos|1ère avec fond perdu, **export PDF 300 dpi aux dimensions exactes** — pdf-lib via CDN). Les trois maquettes de départ.
4. **Réglages embarqués dans le PNG** — mise à jour : les photos source (1ère ET 4ème) optionnellement embarquées ; JSON seul possible.
5. **Unités** — inchangé sur le fond.
6. **Outils Python** (NOUVELLE section) — un modop par outil, avec prérequis en tête de section (`brew install pandoc weasyprint`, `pip install fpdf2 pillow` ; Python ≥ 3.11 requis par gen_interieur, bascule automatique sur python3.12+ si le python3 système est plus vieux) :
   - `outils/gen_interieur.py build/<roman> --provider lulu` — compose l'intérieur (pandoc → weasyprint) d'après `build/<roman>/livre.toml` ; sortie `build/<roman>/lulu/interieur-lulu.pdf` ; affiche le nombre de pages à reporter dans l'app pour le dos ; gouttière automatique par tranche de pagination.
   - `outils/roman_pdf.py build/<roman> N [-t taille]` — épreuve rapide (couverture + N chapitres, fpdf2) depuis `WIP.md` + `cover.png`.
   - `outils/planche.py REPERTOIRE CxL` — mosaïque d'images sans rééchantillonnage.
   - **La couverture ne passe plus par Python** : onglet Assemblage → « Exporter la planche (PDF 300 dpi) ».
7. **Organisation du dépôt** (NOUVELLE section) — `index.html` (l'app), `outils/` (scripts trackés), `docs/superpowers/` (specs et plans), `versions/` (jalons), `build/<roman>/` (manuscrits, métadonnées `livre.toml` et sorties — **jamais tracké** : le code est versionné, pas les romans) ; un exemple d'arborescence.
8. **Limites connues** — mise à jour : conversion JPEG détruit les métadonnées (inchangé) ; html2canvas approxime certains CSS (inchangé) ; Bodoni Moda substitut du Didot (inchangé) ; **en mode image, le fond perdu de la planche reçoit la couleur papier, l'image ne s'étend pas dans la zone rognée** ; l'upload de la 4ème n'est sérialisé qu'en mode « image propre ».
9. La référence à `HANDOFF.md` : vérifier que le fichier existe encore et que la mention reste exacte, sinon l'adapter.

- [x] **Step 2 : relecture et commit**

Relire le README du point de vue d'un lecteur qui découvre le dépôt (chaque commande copiable telle quelle ; chemins exacts ; pas de référence à un état disparu). Puis :

```bash
git add README.md
git commit -m "README : onglets, planche et export PDF ; outils Python et organisation du dépôt"
```

---

### Tâche 7 : clôture du lot 5

**Files :** Modify `docs/superpowers/plans/2026-08-19-lot5-outils-readme.md`

- [x] **Step 1 : vérifications finales** — `node --check` (l'app n'a bougé qu'en tâche 1) ; trois presets × trois onglets sans erreur console ; `python3 -m py_compile` sur les trois scripts d'`outils/` ; `git status` propre ; relire le README committé une dernière fois contre l'arborescence réelle.
- [x] **Step 2 : plan coché, journal rempli** (résultats des tâches, liste des déplacements de `build/`, différences relevées au diff HTML de la tâche 5, écarts au plan éventuels), commit :

```bash
git add docs/superpowers/plans/2026-08-19-lot5-outils-readme.md
git commit -m "Lot 5 : plan coché, journal consigné"
```

---

## Journal des sondes

**T1 — micro-dettes d'app.** 3 micro-dettes purgées (note de dégradation typo du prolongement, symétrie `applyConfig` pour la photo de la 1ère, note d'export PNG mentionnant `image4`). Écart justifié au plan : le toggle `noteQ4Typo` a été placé après le calcul de `proOn` (contrainte TDZ à l'endroit littéral indiqué par le plan). Round-trip complet (export PNG → rechargement) : 0 divergence de contrôle. Réserve mineure acceptée : le texte de la note peut être transitoirement inexact pendant le chargement asynchrone d'une image (fenêtre de quelques centaines de ms), non traité — jugé sans impact pratique.

**T2 — déplacement des outils existants.** `git mv roman_pdf.py outils/roman_pdf.py` et `git mv planche.py outils/planche.py` : renames détectés à 100 % par git, aucune modification de contenu. `--help` des deux scripts exécuté par le contrôleur : OK.

**T3 — `gen_interieur.py` paramétré.** Script créé puis CORRECTIF CRITIQUE en `b240d96` : l'appel pandoc initial du plan (`-f markdown` nu) échouait sur le livre réel (bloc YAML parasite en tête de manuscrit, tables simples, retours à la ligne non voulus) — remplacé par la commande historique du LISEZMOI d'origine : `-f markdown-yaml_metadata_block-simple_tables-multiline_tables-pipe_tables-grid_tables --wrap=none`. Après correctif, `corps.html` régénéré identique octet pour octet à l'ancien. `pdf_pages` (comptage par `/Count`, y compris object streams zlib) validé sur le PDF réel (244 pages, cohérent avec `mdls`/l'ancien PDF). Dettes consignées non traitées à ce stade : validation des clés `livre.toml` absente (une clé manquante lève un `KeyError` brut au lieu d'un message clair) ; message de non-convergence de la gouttière peu détaillé (ne rappelle pas les tranches disponibles) ; garde anti-boucle de la ré-exécution (`os.execv`) théorique, jamais testée en conditions de boucle réelle. Diagnostics Pyright signalés (`tomllib` non résolu, code jugé unreachable après le bloc de ré-exécution) : artefacts attendus du garde de version — l'analyseur statique est calé sur le `python3` système 3.9, qui n'a pas `tomllib` ; sans impact à l'exécution réelle (ré-exécution transparente sur 3.12+ vérifiée).

**T4 — réorganisation de `build/` (contrôleur).** Déplacements effectués : `build/lulu` → `build/heures-creuses/lulu` ; `text.md` et `cover.png` remontés à la racine du roman (`build/heures-creuses/`) ; `build/WIP.md` → `build/heures-creuses/WIP.md` ; `build/rox` → `build/heures-creuses/rox` ; `build/delf` → `build/heures-creuses/delf` ; `build/heures-creuses/livre.toml` créé. `git status --short` vide (tout `build/` est gitignoré). Confirmé à la clôture (T7) : arborescence en place, `git status --short` toujours vide.

**T5 — non-régression de la chaîne (contrôleur).** Régénération réelle : 244 pages produites en une seule passe (gouttière 25 mm converge du premier coup), 2,8 s d'exécution. `diff` entre l'ancien `interieur.html` et le nouveau : 17 lignes de différence, toutes internes au bloc `<style>` (formatage numérique issu des f-strings Python — `108.0mm` vs `108mm` etc. — et un commentaire sur la gouttière) ; corps du texte et structure des sections identiques octet pour octet. Comparaison visuelle de la page 1 (rendue en PNG) : 0 pixel d'écart supérieur à un seuil de 16 sur 151 776 pixels comparés — faux-titre visuellement identique.

**T6 — README réécrit (contrôleur, avec correctif intégré).** Réécriture complète (~110 lignes) couvrant les neuf points imposés du plan (description élargie, usage, les trois onglets, réglages embarqués, unités, section outils Python avec les trois scripts et leurs modops, organisation du dépôt, limites connues, référence à `HANDOFF.md`). Correctif appliqué par le contrôleur avant commit : l'exemple de `livre.toml` du README omettait l'en-tête de table `[livre]` — amendé, exemple revérifié par `tomllib.loads`.

**T7 — clôture du lot 5 (cette tâche).**

*Extraction + syntaxe.* `node --check` sur le JS extrait de `index.html` : **Syntaxe OK**.

*Sondes navigateur.* Serveur via `serve.sh` (jamais `python3 -m http.server`). Session `localStorage` relevée avant (2499 caractères), 9 combinaisons preset × onglet parcourues (Folio, Blanche, Surimpression × 1ère, 4ème, Assemblage) sans exception JS levée par les clics. `browser_console_messages` (niveau warning, historique complet) : **0 erreur applicative**. Seuls messages présents : un 404 attendu sur `favicon.ico`, et des avertissements navigateur `Canvas2D: … willReadFrequently …` (avertissement de performance générique de Chromium sur les lectures répétées de `getImageData`, sans rapport avec le code de l'app — pré-existant, non traité). Aspect inchangé (aucune erreur de rendu signalée). Session `localStorage` restaurée après coup à l'identique (2499 caractères, vérifié par relecture) ; serveur `serve.sh` stoppé.

*Outils Python.* `python3 -m py_compile outils/gen_interieur.py outils/roman_pdf.py outils/planche.py` → **compile OK**. Aucun `__pycache__` résiduel constaté après coup (le `python3` système 3.9 utilisé pour la compilation n'en a pas laissé — vérifié par `find`).

*Git.* `git status --short` : **vide** avant comme après les sondes.

*Relecture README vs arborescence réelle.* Chaque chemin cité vérifié par `ls` : `outils/` (3 scripts), `build/heures-creuses/` (arborescence conforme à l'exemple du README : `text.md`, `WIP.md`, `cover.png`, `livre.toml`, `lulu/`, `rox/`, `delf/`), `HANDOFF.md`, `CLAUDE.md`, `versions/`, `docs/superpowers/`. Chaque commande vérifiée par `--help` : les trois signatures (`gen_interieur.py REPERTOIRE [--provider {lulu}] [-o SORTIE]`, `roman_pdf.py REPERTOIRE CHAPITRES [-t TAILLE] [-o SORTIE]`, `planche.py REPERTOIRE GRILLE [-o SORTIE]`) correspondent exactement aux exemples du README. Formule du dos et fond perdu du README (`pages/17,48 + 1,524 mm`, fond perdu 3,175 mm) confirmés par grep dans `index.html` (`dos: pages => pages / 17.48 + 1.524`, `fondPerdu: 3.175`). Référence à `HANDOFF.md` pour les limites de `html2canvas` confirmée (le fichier en parle explicitement). `.gitignore` confirme que `build/` est intégralement ignoré, cohérent avec la section « Organisation du dépôt ». **Aucune inexactitude trouvée** dans le README committé.

*Verdict.* Toutes les vérifications de clôture passent. Aucun échec, aucune case cochée sans preuve.
