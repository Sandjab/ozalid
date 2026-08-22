# Ozalid — instructions

Chaîne d'auto-édition, du manuscrit aux packages prestataires : intérieur composé,
couverture, planche, dos qui découle de la pagination sans jamais être ressaisi.
**L'application qui fait foi est Ozalid Studio, dans `app/`** — Tauri 2 + Rust, front
vanilla sans bundler, Typst en sidecar. Son architecture, sa mise en route et le plan
du `.ozalid` sont dans `app/README.md`.

Le reste de la racine est **gelé** (spec `2026-08-20-ozalid-studio-design.md`) :
`index.html` est l'ancien atelier de couverture, `outils/` l'ancienne chaîne Python
(pandoc + WeasyPrint). On n'y touche que sur demande explicite ; leurs conventions
sont conservées en fin de fichier.

`build/` n'est jamais tracké : ressources partagées dans `build/in/{covers,texts,editors}/`,
un répertoire de travail par combinaison texte × couverture (au minimum un `livre.toml`,
dont les chemins partent de `build/`). Le `livre.toml` fait foi pour l'identité du livre
à l'import ; ensuite c'est le `.ozalid` qui la porte.

**Français** dans l'interface, les commentaires et les commits. Termes techniques
anglais conservés tels quels (`fond perdu` reste `fond perdu`, mais `viewport`,
`chunk`, `canvas` ne se traduisent pas).

## Vérifications avant commit (`app/`)

- `cargo fmt --check` et `cargo clippy --all-targets -- -D warnings`, propres.
- `cargo test` (depuis `app/src-tauri/`) et `node --test tests/*.test.js` (depuis `app/`).
- `cargo run --example temoin` si un fichier de `app/src-tauri/` a changé : le compte
  de pages affiché est le témoin de non-régression, à comparer au précédent sur le
  même manuscrit.
- Tout test neuf doit avoir été **vu échouer** — TDD ou mutation ciblée (spec § 7 de
  chaque chantier) : un test qui n'a jamais été rouge ne protège rien.

## Pièges connus (`app/`)

- Typst est lancé avec `--ignore-system-fonts` : une famille absente des répertoires
  embarqués ne fait pas échouer la composition, elle passe en écriture de repli —
  signalé au compte rendu depuis `typst::compile`, mais en dev `target/debug/fonts`
  ne suit pas `fonts/` tout seul.
- La version de Typst est épinglée : deux versions ne composent pas forcément le même
  nombre de pages, donc pas le même dos. La relever est un changement délibéré, à
  revalider sur un manuscrit réel.

---

## L'atelier de couverture gelé (`index.html`)

Un seul fichier HTML autonome, sans build : trois onglets (1ère, 4ème, Assemblage) et
export PDF de la planche. Les sorties vivaient dans `build/<travail>/out/` et les
packages éditeur dans `out/<éditeur>/`. Le titre et l'auteur embarqués dans le PNG
exporté sont un rendu, jamais une source — aucun script ne lit ce chunk.

### Contraintes non négociables

- **Fichier unique.** CSS et JS inline dans `index.html`. Pas de bundler, pas de
  `node_modules`, pas de serveur. Le fichier doit s'ouvrir en `file://`.
- **`localStorage` limité à une seule clé** (`atelier-couverture-session`) : la
  dernière configuration, sauvegardée depuis `render()` (debounce), rechargée au
  démarrage, effacée par « Réinitialiser l'atelier ». Aucun autre usage de
  `localStorage`/`sessionStorage` ; tout le reste de l'état vit en mémoire.
- **Dépendances externes** : Google Fonts, `html2canvas` et `pdf-lib` via CDN. Ne pas
  en ajouter sans raison forte.
- **Tout réglage est en pourcentage de la largeur de couverture**, jamais en px
  absolus. C'est ce qui rend les maquettes portables d'un format à l'autre.

### Architecture

1. **`render()`** — fonction unique qui lit tous les contrôles et écrit des variables
   CSS sur `#cover`, `#cover4` et `#plancheFp`. Aucun autre endroit ne touche au style
   de la couverture ; seule exception établie : `buildPlanche` (appelée par `render()`
   et par l'export). Tout nouveau réglage passe par `render()`.
2. **Presets** — objet `PRESETS`, une clé par maquette, mappant `id de contrôle →
   valeur`. Ajouter une maquette = une entrée et un bouton.
3. **Sérialisation PNG** — `collectConfig` / `applyConfig` balaient les contrôles dont
   l'id commence par `in` : **tout contrôle nommé `inXxx` est automatiquement
   sauvegardé**, un contrôle nommé autrement est silencieusement perdu à l'export.

Ajouter un réglage : un `<input id="inXxx">` (avec `<span class="val" id="vXxx">` si
range), une ligne dans `render()`, une entrée dans l'objet `R` de `render()`, une
valeur dans chaque preset. Rien d'autre — la persistance suit.

### Vérifications et pièges

- `node --check` sur le JS extrait ; round-trip métadonnées (exporter un PNG, le
  recharger, tous les contrôles reviennent à l'identique) ; les trois presets × les
  trois onglets après toute modification de `render()` ; vérifier le rendu **à
  l'export**, pas seulement à l'écran — `html2canvas` ne reproduit pas tout le CSS.
- Le cadre est fait de `<div>` imbriquées à bordures en px calculées, pas d'`outline`
  ni de `box-shadow` — `html2canvas` les rend mal.
- `aspect-ratio` sur `.cover` est la source de vérité du format ; pas de hauteur en
  dur. `fitCover()` après tout changement de format, avant `render()`.
