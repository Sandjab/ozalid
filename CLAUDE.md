# Ozalid — instructions

Atelier de packaging de couverture pour l'auto-édition : l'app est un seul fichier HTML autonome, sans build (trois onglets — 1ère, 4ème, Assemblage — et export PDF de la planche). La chaîne Python de composition de l'intérieur vit dans `outils/` (voir README) ; `build/` n'est jamais tracké : ressources partagées dans `build/in/{covers,texts,editors}/`, un répertoire de travail par combinaison texte × couverture (au minimum un `livre.toml`, dont les chemins partent de `build/`), sorties dans son `out/` et packages éditeur dans `out/<éditeur>/`. Le `livre.toml` fait foi pour l'identité du livre ; le titre et l'auteur embarqués dans le PNG sont un rendu, jamais une source — aucun script ne lit ce chunk.

## Contraintes non négociables

- **Fichier unique.** CSS et JS inline dans `index.html`. Pas de bundler, pas de `node_modules`, pas de serveur. Le fichier doit s'ouvrir en `file://`.
- **`localStorage` limité à une seule clé** (`atelier-couverture-session`) : la dernière configuration, sauvegardée depuis `render()` (debounce), rechargée au démarrage, effacée par « Réinitialiser l'atelier » dans le menu Réglages. Aucun autre usage de `localStorage`/`sessionStorage` ; tout le reste de l'état vit en mémoire.
- **Dépendances externes** : Google Fonts, `html2canvas` et `pdf-lib` via CDN. Ne pas en ajouter sans raison forte.
- **Tout réglage est en pourcentage de la largeur de couverture**, jamais en px absolus. C'est ce qui rend les maquettes portables d'un format à l'autre.
- **Français** dans l'interface, les commentaires et les commits. Termes techniques anglais conservés tels quels (`fond perdu` reste `fond perdu`, mais `viewport`, `chunk`, `canvas` ne se traduisent pas).

## Architecture

Trois blocs dans le `<script>` final :

1. **`render()`** — fonction unique qui lit tous les contrôles et écrit des variables CSS sur `#cover`, `#cover4` et `#plancheFp`. Aucun autre endroit ne doit toucher au style de la couverture ; seule exception établie : `buildPlanche` (appelée par `render()` et par l'export), qui pose l'échelle de la planche et les px inline de ses clones. Tout nouveau réglage passe par `render()`.
2. **Presets** — objet `PRESETS`, une clé par maquette, mappant `id de contrôle → valeur`. Ajouter une maquette = ajouter une entrée et un bouton.
3. **Sérialisation PNG** — `collectConfig` / `applyConfig` balaient le DOM du panneau (les `input`, `select` et `textarea` dont l'id commence par `in`), donc **tout contrôle nommé `inXxx` est automatiquement sauvegardé**. Un contrôle nommé autrement sera silencieusement perdu à l'export.

## Ajouter un réglage

1. Un `<input id="inQuelqueChose">` dans le panneau, avec un `<span class="val" id="vQuelqueChose">` pour la lecture si c'est un range.
2. Une ligne dans `render()` qui écrit la variable CSS correspondante.
3. Une entrée dans l'objet `R` de `render()` pour la lecture affichée.
4. Une valeur dans chaque preset de `PRESETS`.

Rien d'autre. La persistance suit automatiquement.

## Vérifications avant commit

- `node --check` sur le JS extrait — la syntaxe doit passer.
- Round-trip métadonnées : exporter un PNG, le recharger, vérifier que tous les contrôles reviennent à l'identique.
- Tester les trois presets × les trois onglets (1ère, 4ème, Assemblage) après toute modification de `render()`.
- Vérifier le rendu à l'export, pas seulement à l'écran : `html2canvas` ne reproduit pas tout le CSS fidèlement.
- `python3 -m py_compile outils/*.py` si un script d'`outils/` a changé, et la chaîne doit régénérer un intérieur complet sur un répertoire de travail réel (`outils/gen_interieur.py build/<travail> --provider lulu`) : le compte de pages affiché est le témoin de non-régression, à comparer au précédent sur le même manuscrit.

## Pièges connus

- Le cadre est fait de `<div>` imbriquées avec des bordures en px calculées, pas de `outline` ni de `box-shadow` — `html2canvas` rend mal ces derniers.
- `aspect-ratio` sur `.cover` est la source de vérité du format. Ne pas fixer de hauteur en dur.
- `fitCover()` doit être appelée après tout changement de format, avant `render()`.
