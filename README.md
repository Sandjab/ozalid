# Ozalid

Atelier de packaging de couverture pour l'auto-édition : maquette de première, quatrième de couverture, planche complète avec dos, export au format attendu par le prestataire d'impression. Outil HTML autonome, sans build, sans dépendance serveur.

Le nom vient du terme de prépresse désignant l'épreuve de contrôle avant tirage.

## Usage

Ouvrir `index.html` dans un navigateur. Rien à installer. Les outils Python (composition de l'intérieur, épreuves de lecture) sont optionnels — voir leur section.

## Ce que ça fait

Trois onglets :

- **1ère** — trois modes de mise en page : **Bandeau** (bande de titre en haut, image à fond perdu en dessous, archétype Folio / Penguin Modern Classics), **Surimpression** (image sur toute la surface, texte par-dessus, voile de lisibilité réglable), **Sans image** (composition purement typographique, archétype Blanche / NRF). Un générateur de cadre indépendant du mode reproduit le triple filet Gallimard, paramétrable sur six axes.
- **4ème** — texte de présentation, pied, zone ISBN laissée vide (le code-barres est ajouté par le prestataire). Quatre fonds au choix : papier de la 1ère, couleur distincte, image propre, prolongement panoramique de l'image de la 1ère.
- **Assemblage** — planche complète 4ème | dos | 1ère au gabarit du prestataire (Lulu : dos = pages/17,48 + 1,524 mm, fond perdu 3,175 mm). Le nombre de pages de l'intérieur se saisit dans l'onglet ; « Exporter la planche (PDF 300 dpi) » produit un PDF aux dimensions millimétriques exactes (pdf-lib via CDN), le fichier à téléverser chez le prestataire.

Trois maquettes de départ préchargées : `Folio`, `Blanche`, `Surimpression`. Chacune recharge l'intégralité des réglages.

## Réglages embarqués dans le PNG

À l'export, la configuration complète est écrite dans un chunk `tEXt` du PNG sous la clé `atelier-couverture`, avec optionnellement les photos source (1ère et 4ème) rééchantillonnées. Recharger le PNG dans l'outil restaure la maquette entière.

Le fichier reste un PNG standard : lisible par n'importe quel visualiseur, PIL et `exiftool` voient le bloc.

Les réglages peuvent aussi être exportés seuls en JSON — plus léger, versionnable, lisible en diff.

## Unités

Corps de texte, filets et marges sont exprimés en pourcentage de la largeur de couverture. Changer de format ne casse aucun réglage typographique.

## Outils Python

Prérequis : `brew install pandoc weasyprint` et `pip install fpdf2 pillow`. `gen_interieur.py` exige Python ≥ 3.11 (`tomllib`) et bascule seul sur `python3.11` ou plus récent si le `python3` système est plus vieux.

### Intérieur du roman

```
python3 outils/gen_interieur.py build/mon-roman --provider lulu
```

Compose l'intérieur (pandoc → weasyprint) d'après `build/mon-roman/livre.toml` et le manuscrit qu'il désigne. Format du manuscrit : titre en `# `, chapitres en `## NN - Titre`, séparateurs de scène `---`. Sortie : `build/mon-roman/lulu/interieur-lulu.pdf`. La gouttière dépend de la tranche de pagination : une seconde passe recompose automatiquement si le compte de pages sort de la tranche supposée. Le nombre de pages final est affiché — à reporter dans l'onglet Assemblage pour le calcul du dos.

Exemple de `livre.toml` :

```toml
[livre]
titre = "Mon roman"
titre_page = "Mon\nroman"      # optionnel ; \n = saut de ligne sur la page de titre
auteur = "Prénom Nom"
genre = "roman"
copyright = """© Prénom Nom, 2026.
Tous droits réservés."""
chapitres = 40
manuscrit = "text.md"
```

### Épreuve de lecture

```
python3 outils/roman_pdf.py build/mon-roman 12 -t 10
```

Génère un PDF de lecture au format poche (fpdf2) depuis `WIP.md` et `cover.png` du répertoire : couverture + les 12 premiers chapitres. Sortie : `roman.pdf` dans le répertoire ; `-t` règle le corps du texte en points.

### Planche d'images

```
python3 outils/planche.py images 3x2
```

Assemble les images d'un répertoire en une mosaïque PNG sans redimensionnement (PIL). `3x2` = colonnes × lignes ; sortie : `planche-3x2.png` dans le répertoire.

La couverture, elle, ne passe plus par Python : onglet Assemblage → « Exporter la planche (PDF 300 dpi) ».

## Organisation du dépôt

```
index.html              l'application, autonome
outils/                 scripts Python trackés
docs/superpowers/       specs et plans de développement
versions/               jalons historiques, chaque fichier autonome
build/<roman>/          manuscrits, livre.toml et sorties — jamais tracké
HANDOFF.md              état du code, décisions, dettes, pistes
CLAUDE.md               instructions pour Claude Code
```

Le code est versionné, pas les romans : `build/` reste hors git. Arborescence type d'un roman :

```
build/mon-roman/
  text.md          manuscrit complet
  WIP.md           version de travail pour les épreuves
  cover.png        couverture exportée depuis l'app
  livre.toml       métadonnées du livre
  lulu/            intérieur composé pour Lulu
  rox/  delf/      épreuves destinées aux relecteurs
```

## Limites connues

- Une conversion vers JPEG détruit les métadonnées. Conserver le JSON en parallèle si les fichiers passent dans un pipeline de compression.
- Le rendu PNG passe par `html2canvas`, qui approxime certaines propriétés CSS. Voir `HANDOFF.md`.
- Le Didot original de la collection Blanche n'existe pas en version numérique. Bodoni Moda sert de substitut.
- Sur la planche, en mode image, le fond perdu reçoit la couleur papier : l'image ne s'étend pas dans la zone rognée.
- L'image de la 4ème n'est embarquée dans le PNG qu'en mode « image propre ».
