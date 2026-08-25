# Ozalid

> **Dépôt gelé.** Le développement continue sur
> [**OzalidStudio**](https://github.com/Sandjab/OzalidStudio), qui porte
> l'application de bureau (Tauri, macOS + Windows) née dans `app/`. Ce dépôt
> conserve l'atelier HTML historique, les outils Python et l'historique complet ;
> il n'évoluera plus.

Atelier de packaging de couverture pour l'auto-édition : maquette de première, quatrième de couverture, planche complète avec dos, export au format attendu par le prestataire d'impression. Outil HTML autonome, sans build, sans dépendance serveur.

Le nom vient du terme de prépresse désignant l'épreuve de contrôle avant tirage.

## Usage

Ouvrir `index.html` dans un navigateur. Rien à installer. Les outils Python (composition de l'intérieur, épreuves de lecture) sont optionnels — voir leur section.

Pour la marche à suivre complète, du manuscrit au fichier téléversé chez l'imprimeur : `COOKBOOK.md`.

## Ce que ça fait

Trois onglets :

- **1ère** — trois modes de mise en page : **Bandeau** (bande de titre en haut, image à fond perdu en dessous, archétype Folio / Penguin Modern Classics), **Surimpression** (image sur toute la surface, texte par-dessus, voile de lisibilité réglable), **Sans image** (composition purement typographique, archétype Blanche / NRF). Un générateur de cadre indépendant du mode reproduit le triple filet Gallimard, paramétrable sur six axes.
- **4ème** — texte de présentation, pied, zone ISBN laissée vide (le code-barres est ajouté par le prestataire). Quatre fonds au choix : papier de la 1ère, couleur distincte, image propre, prolongement panoramique de l'image de la 1ère.
- **Assemblage** — planche complète 4ème | dos | 1ère au gabarit du prestataire (Lulu : dos = pages/17,48 + 1,524 mm, fond perdu 3,175 mm ; BoD : dos = pages × 0,0675 + 0,6 mm en crème 90 g, fond perdu 5 mm ; KDP : dos = pages × 0,0635 mm en crème ou × 0,0572 mm en blanc, fond perdu 3,175 mm). Un prestataire **« Dos mesuré »** couvre ceux qui ne publient pas de formule et fournissent un gabarit — CoolLibri, TheBookEdition, Bookvault : le dos et le fond perdu s'y saisissent tels que relevés. Le nombre de pages de l'intérieur se saisit dans l'onglet ; « Exporter la planche (PDF 300 dpi) » produit un PDF aux dimensions millimétriques exactes (pdf-lib via CDN), le fichier à téléverser chez le prestataire.

Trois maquettes de départ préchargées : `Folio`, `Blanche`, `Surimpression`. Chacune recharge l'intégralité des réglages.

## Réglages embarqués dans le PNG

À l'export, la configuration complète est écrite dans un chunk `tEXt` du PNG sous la clé `atelier-couverture`, avec optionnellement les photos source (1ère et 4ème) rééchantillonnées. Recharger le PNG dans l'outil restaure la maquette entière.

Le fichier reste un PNG standard : lisible par n'importe quel visualiseur, PIL et `exiftool` voient le bloc.

Les réglages peuvent aussi être exportés seuls en JSON — plus léger, versionnable, lisible en diff.

## Unités

Corps de texte, filets et marges sont exprimés en pourcentage de la largeur de couverture. Changer de format ne casse aucun réglage typographique.

## Outils Python

Prérequis : `brew install pandoc weasyprint` et `pip install fpdf2 pillow`. Les deux scripts lisent `livre.toml` : `gen_interieur.py` exige Python ≥ 3.11 (`tomllib`) et bascule seul sur `python3.11` ou plus récent si le `python3` système est plus vieux ; `roman_pdf.py` reste sur l'interpréteur qui porte `fpdf2` et se rabat sur `tomli` (`pip install tomli`) quand `tomllib` manque.

### Intérieur du roman

```
python3 outils/gen_interieur.py build/mon-roman --provider lulu
```

Compose l'intérieur (pandoc → weasyprint) d'après `build/mon-roman/livre.toml` et le manuscrit qu'il désigne. `--provider` accepte `lulu` (poche 108 × 175), `bod` (13,5 × 21,5 cm) les trois formats KDP outillés : `kdp-5x8` (127 × 203,2), `kdp-55x85` (139,7 × 215,9), `kdp-6x9` (152,4 × 228,6), et les trois formats roman de CoolLibri : `coollibri-110x170`, `coollibri-148x210`, `coollibri-160x240`. Le format composé doit être celui choisi dans l'app — les deux tables sont séparées, rien ne les recoupe. Format du manuscrit : titre en `# `, chapitres en `## NN - Titre`, séparateurs de scène `---`. Sortie : `build/mon-roman/out/lulu/interieur-lulu.pdf`. La gouttière dépend de la tranche de pagination : une seconde passe recompose automatiquement si le compte de pages sort de la tranche supposée. Un compte impair est corrigé de la même façon, par une page blanche de fin sans folio — une feuille porte deux pages, les prestataires refusent l'impair. Le nombre de pages final est affiché — à reporter dans l'onglet Assemblage pour le calcul du dos.

Exemple de `livre.toml` :

```toml
[livre]
titre = "Mon roman"
titre_page = "Mon\nroman"      # optionnel ; \n = saut de ligne sur la page de titre
auteur = "Prénom Nom"
genre = "roman"
copyright = """© Prénom Nom, 2026.
Tous droits réservés."""
chapitres = 40                          # facultatif ; contrôle d'intégrité au gel
manuscrit = "in/texts/mon-roman.md"     # depuis build/ ; chemin absolu accepté
couverture = "in/covers/mon-roman.png"
```

`chapitres` se déduit du texte : la clé ne sert qu'à figer le compte quand le manuscrit ne doit plus bouger, et le script refuse alors de composer s'il en trouve un autre. Tant qu'on écrit, autant l'omettre — le compte trouvé est affiché à chaque composition.

`manuscrit` et `couverture` sont les seules entrées que lisent les scripts : pour tirer une épreuve d'un autre état du texte, on change la clé, pas le script. Leurs chemins partent de `build/`, ce qui permet à plusieurs répertoires de travail de partager le même manuscrit ; un chemin absolu est pris tel quel.

### Épreuve de lecture

```
python3 outils/roman_pdf.py build/mon-roman 12 -t 10
```

Génère un PDF de lecture au format poche (fpdf2) depuis le manuscrit et la couverture désignés par `livre.toml` : couverture + les 12 premiers chapitres. Sortie : `build/mon-roman/out/roman.pdf` — l'épreuve ne vise aucun éditeur, elle reste à la racine de `out/` ; `-t` règle le corps du texte en points.

Celle-ci sert à **faire lire** : format poche, couverture pleine page, les premiers chapitres. À ne pas confondre avec l'épreuve de relecture d'Ozalid Studio, qui est un autre document et ne la remplace pas : A4, le manuscrit entier, fer à gauche, une marge de 50 mm et des numéros de ligne pour **faire corriger**. Voir `app/README.md`.

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
build/                  ressources et répertoires de travail — jamais tracké
COOKBOOK.md             publier pas à pas, un chapitre par prestataire
NOTES.md                origine du projet, analyse de la Blanche, juridique
CLAUDE.md               instructions pour Claude Code
```

Le code est versionné, pas les romans : `build/` reste hors git. Il se lit en deux temps — les ressources partagées d'un côté, les répertoires de travail de l'autre :

```
build/
  in/                     ressources partagées
    covers/               premières de couverture exportées depuis l'app (PNG)
    texts/                manuscrits (Markdown)
    editors/              guides de composition des éditeurs (PDF)
  mon-roman/              un répertoire de travail…
    livre.toml            …contient au minimum son livre.toml
    out/                  tout ce qui est produit
      roman.pdf           épreuve de lecture, sans éditeur visé
      lulu/               package d'un éditeur : intérieur, HTML intermédiaires
```

Un répertoire de travail est une **combinaison** : le même manuscrit et deux couvertures différentes font deux répertoires, chacun avec son `livre.toml`. Ce qui est partagé vit dans `in/`, ce qui est produit vit dans `out/`, ce qui est spécifique à un éditeur vit dans `out/<éditeur>/`.

### Qui porte quoi

Le `livre.toml` fait foi pour l'identité du livre : titre, auteur, genre, copyright, nombre de chapitres. Le PNG de couverture embarque lui aussi un titre et un auteur, mais **comme rendu, pas comme source** — sa casse est celle de la maquette (`inTitleCase`), et les scripts ne le lisent jamais. Un titre corrigé se corrige dans le TOML ; la couverture, elle, se refait dans l'app.

## Limites connues

- Une conversion vers JPEG détruit les métadonnées. Conserver le JSON en parallèle si les fichiers passent dans un pipeline de compression.
- Le rendu PNG passe par `html2canvas`, qui approxime certaines propriétés CSS. Voir `NOTES.md`.
- Le Didot original de la collection Blanche n'existe pas en version numérique. Bodoni Moda sert de substitut.
- Sur la planche, le fond perdu prolonge la couleur de fond de chaque panneau (4ème, dos, 1ère), jamais l'image : en mode image, la photo s'arrête au trait de coupe.
- L'image de la 4ème n'est embarquée dans le PNG qu'en mode « image propre ».
