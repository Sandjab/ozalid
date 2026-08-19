# Image de fond de la 4ème, dette lot 3, outils Python et README

Date : 2026-08-19
Statut : validé (brainstorming)

## Objectif

Deux volets. **Volet app (lot 4)** : purger la dette technique du lot 3 puis
donner à la 4ème de couverture un fond image — image propre ou prolongement
panoramique de la 1ère. **Volet chaîne & docs (lot 5)** : sortir la chaîne
Python de composition de l'intérieur du répertoire non tracké `build/`, la
paramétrer pour plusieurs romans et plusieurs prestataires, et remettre le
README au niveau de ce que le projet fait réellement.

## Décisions de cadrage (brainstorming du 19/08)

- **Image de la 4ème : les deux modes.** `image` (upload dédié) **et**
  `prolongement` (panorama depuis la 1ère). Sélecteur `inQ4BgMode` étendu :
  `herite | couleur | image | prolongement`.
- **Prolongement : 1ère maître, extension à gauche.** La géométrie de l'image
  de la 1ère ne bouge jamais ; dos et 4ème montrent la continuation de la photo
  vers la gauche. Là où la matière manque, le papier apparaît (signalé à
  l'écran). En mode typo (pas d'image sur la 1ère), le prolongement dégénère en
  fond papier + note.
- **Architecture du prolongement : tranches par panneau.** Chaque panneau
  (4ème, dos) porte sa propre `<img>` positionnée pour montrer sa tranche du
  panorama, calculée avec les formules d'`artFreezeCss` décalées
  horizontalement. Pas d'image unique posée sous la planche.
- **Mode `image` : jeu complet de réglages**, en variantes `inQ4Xxx` du
  fieldset Image de la 1ère (cadrage X/Y, échelle, proportions conservées,
  déformation, voile + intensité, remplacement de fichier).
- **`gen_couverture.py` retiré de la chaîne.** Depuis le lot 3, la planche de
  couverture sort de l'app (onglet Assemblage → PDF 300 dpi). Le script reste
  en fin de vie dans `build/`, non migré, non documenté.
- **Arborescence : `outils/` tracké + `build/<roman>/` ignoré.** Les trois
  scripts Python vivent dans `outils/` ; `build/` reste intégralement
  gitignoré et se réorganise par roman.
- **Paramétrage : `livre.toml` par roman + presets provider dans le script.**
- **Orchestration : `gen_interieur.py` enchaîne pandoc → composition →
  weasyprint** en une commande.

## Volet app (lot 4)

### 1. Dette du lot 3, purgée en tête de lot

a. **`plancheDims()`** : helper unique (provider, pages clampées, dos mm,
   largeur/hauteur mm de la planche) consommé par `render()` et l'écouteur
   `btnPlanche` — fin des formules en trois exemplaires.
b. **Capture pré-rendu** : les valeurs d'export (arrêts du dégradé de fond
   perdu, géométries d'images figées) sont calculées une fois, juste après le
   `buildPlanche` d'export ; `onclone` reçoit une fermeture qui applique ces
   valeurs sans relire le DOM vivant. Supprime la fenêtre de course relevée en
   revue finale du lot 3.
c. **`s4.backgroundColor`** remplacé par une variable CSS posée par `render()`
   (même motif que `--dos-bg`).
d. **Commentaire CSS « (tâche 4) »** du bloc planche toiletté (référence de
   lot ambiguë).

### 2. Fond image de la 4ème

- **DOM** : `#cover4` reçoit une zone `div.art` + `<img>` (ids dédiés, ex.
  `art4`/`elImg4`) sous les couches texte/pied/ISBN. Le `#dos` reçoit une
  `<img>` de tranche, présente uniquement en prolongement.
- **Mode `image`** : image propre uploadée (`inQ4File`), plein fond de la
  4ème, voile de lisibilité par-dessus, sous le texte. Sans image chargée :
  fond papier + note discrète « choisir une image ». Pas de seconde image par
  défaut embarquée dans le fichier.
- **Mode `prolongement`** : la zone image de la 1ère est répliquée à
  l'identique (position et hauteur — y compris en mode bandeau, où la bande
  d'image continue à la même hauteur) sur le dos et la 4ème, avec le décalage
  horizontal du panorama (dos puis une largeur de couverture, en unités de
  `--cw`). La source est l'image de la 1ère. Le voile de la 4ème s'applique
  aussi en prolongement ; le texte du dos passe au-dessus de sa tranche.
- **Rendu** : `render()` reste l'unique écrivain — il pose les variables et
  calcule les positions de tranches en px à partir de `--cw`, des réglages et
  des dimensions naturelles de l'image (jamais de `getBoundingClientRect` sur
  un panneau potentiellement masqué). Recalcul au chargement de l'image.
- **Sérialisation** : `cfg.image4` via `shrinkSource` généralisé (paramètre
  image source), embarquée dans le PNG et la session sous la même case
  « Y joindre la photo source » ; restaurée par `applyConfig` ; comptée dans le
  round-trip. Tous les nouveaux contrôles en `inQ4Xxx` → persistance
  automatique. Poids : deux images rééchantillonnées à 1600 px restent loin du
  quota `localStorage` (~5 Mo) ; à vérifier en sonde.
- **Presets** : les trois maquettes gardent `inQ4BgMode:'herite'` et reçoivent
  les nouvelles clés à leurs valeurs neutres — aucune maquette ne change
  d'aspect.
- **Export/planche** : `artFreezeCss` généralisé (source, réglages et décalage
  en paramètres, défauts = 1ère) ; `preparePlancheClone` fige les images des
  trois panneaux ; `buildPlanche` remet à l'échelle les px inline des tranches
  comme il le fait pour le cadre. En prolongement, le fond perdu reste rempli
  par le dégradé du lot 3 là où l'image ne fournit pas de matière.

## Volet chaîne & docs (lot 5)

### 3. Répertoire `outils/` (tracké)

- `outils/roman_pdf.py` et `outils/planche.py` : déplacés depuis la racine
  (`git mv`), inchangés.
- `outils/gen_interieur.py` : migré depuis `build/lulu/src/`, paramétré.
  - Appel : `outils/gen_interieur.py build/<roman> [--provider lulu]`.
  - Lit `build/<roman>/livre.toml` (`tomllib`, stdlib ≥ 3.11) : titre, auteur,
    genre, copyright, nombre de chapitres attendu, nom du fichier manuscrit.
    Les pages liminaires (faux-titre, page de titre, copyright) sont générées
    depuis ces champs.
  - `PROVIDERS` dans le script (miroir de celui de l'app) : format de rognage,
    marges, gouttière par tranche de pagination, folio. Lulu seul au départ.
  - Orchestre pandoc → composition → weasyprint (`subprocess`), vérifie la
    présence des outils avec un message clair, garde les HTML intermédiaires à
    côté des sorties, affiche le nombre de pages final (à reporter dans l'app
    pour le calcul du dos). Seconde passe automatique si le compte de pages
    sort de la tranche de gouttière supposée.
  - Sorties dans `build/<roman>/<provider>/`.

### 4. Réorganisation de `build/` (toujours intégralement gitignoré)

Convention `build/<roman>/` : manuscrit, `cover.png`, `livre.toml`, un
sous-répertoire par usage pour les sorties (`lulu/`, `epreuve/`…). Migration
locale des données existantes pendant le lot, déplacements listés un à un :
`build/lulu` → `build/heures-creuses/` (+ `lulu/` pour les livrables), `rox` et
`delf` regroupés en épreuves du même roman. Aucun changement de `.gitignore`.

### 5. README

- Rafraîchissement du volet app : trois onglets (1ère, 4ème, Assemblage),
  planche Lulu (calcul du dos, fond perdu), export PDF 300 dpi (pdf-lib en
  dépendance CDN), les deux modes d'image de la 4ème, limites mises à jour
  (compromis fond perdu en mode image).
- **Nouvelle section « Outils Python »** : un mode opératoire par script
  (commande type, entrées/sorties, prérequis — pandoc, weasyprint, fpdf2,
  Pillow).
- **Nouvelle section « Organisation du dépôt »** : convention
  `build/<roman>/`, ce qui est tracké et pourquoi ; modop couverture pointant
  vers l'app.
- Ton et structure existants conservés.

## Risques identifiés

- **Prolongement sans matière** : photo trop étroite pour couvrir dos + 4ème →
  papier visible à gauche. Assumé, signalé à l'écran ; pas de miroir ni
  d'étirement automatique.
- **Deux images en session** : quota `localStorage` à sonder (attendu très
  en-deçà de 5 Mo).
- **html2canvas et tranches** : les tranches sont des `<img>` positionnées en
  px inline — le chemin le plus fiable connu (même mécanisme que le gel
  existant), mais l'export planche des deux nouveaux modes doit être contrôlé
  visuellement à 300 dpi.
- **Non-régression de la chaîne intérieur** : `gen_interieur.py` paramétré
  doit régénérer l'intérieur du livre réel à l'identique — même compte de
  pages (244) que `interieur-poche.pdf`, contrôle visuel par échantillon.

## Découpage en lots

4. **Lot 4 — app** : dette lot 3 (§1) puis image de fond de la 4ème (§2).
5. **Lot 5 — chaîne & docs** : `outils/` (§3), réorganisation `build/` (§4),
   README complet en dernier (§5).

Chaque lot livre séparément et passe les vérifications du projet : syntaxe
(`node --check`), trois presets et trois modes, round-trip des métadonnées
(incluant `image4`), contrôle du rendu à l'export et pas seulement à l'écran.
