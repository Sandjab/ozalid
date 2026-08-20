# Ozalid Studio

Application de bureau macOS + Windows qui tient la chaîne entière : manuscrit →
intérieur composé → couverture → packages prestataires. Elle succède à la paire
`index.html` + `outils/`, désormais gelée (voir
`docs/superpowers/specs/2026-08-20-ozalid-studio-design.md`).

Ce qu'elle règle : le nombre de pages ne transite plus par un humain. L'intérieur
le produit, la couverture le consomme, et le dos suit le manuscrit sans ressaisie.

**État : jalon 3** — projet `.ozalid`, import d'un livre existant, composition de
l'intérieur, moteur de couverture avec aperçu. Pas encore d'assemblage de planche
ni de packages.

## Stack

- **Tauri 2 + Rust** pour le client, front vanilla sans bundler ni framework.
- **Typst** en sidecar : un binaire statique, sans dépendance système, la même
  version sur les deux plateformes. C'est ce qui rend la pagination reproductible
  d'une machine à l'autre — ni Python, ni pandoc, ni WeasyPrint.

## Mise en route

```
app/outils/typst.sh --local     # ou sans --local pour télécharger la version épinglée
app/outils/polices.sh           # ~6 Mo de polices OFL
cd app/src-tauri && cargo tauri dev
```

`typst.sh` place le sidecar dans `src-tauri/binaries/`, `polices.sh` les polices
dans `src-tauri/fonts/` — deux répertoires non versionnés. La version de Typst est
**épinglée** : deux versions ne composent pas forcément le même nombre de pages,
donc pas le même dos. La relever est un changement délibéré, à revalider sur un
manuscrit réel.

Typst est lancé avec `--ignore-system-fonts` : seules les polices embarquées
comptent, sans quoi une police du poste pourrait s'y substituer et le rendu
dépendrait de la machine.

## Modules

L'interface n'a aucune logique métier : elle invoque des commandes et affiche des
résultats. Tout le reste est testable sans fenêtre.

| Module | Rôle |
|---|---|
| `providers` | Table **unique** des gabarits : format, marges, gouttières, fond perdu, formule de dos |
| `manuscrit` | Markdown → chapitres → contenu Typst, avec refus explicite du non composable |
| `projet` | Le `.ozalid` : lecture, écriture, identité du livre |
| `png` | Lecture du bloc de réglages qu'`index.html` écrit dans ses PNG |
| `import` | Un `livre.toml` et un PNG de l'atelier → un projet et sa maquette |
| `image` | Dimensions naturelles d'une image, et cadrage dans une zone |
| `couverture` | Maquette typée → source Typst des deux faces |
| `maquettes` | Folio, Blanche et Surimpression |
| `typst` | Invocation du sidecar : mesurer la pagination, compiler, rendre un aperçu |
| `interieur` | Source Typst de l'intérieur, et convergence gouttière/parité |
| `commands` | Frontière avec l'interface, et projet ouvert |

`providers` fusionne les deux tables historiques du projet — celle d'`index.html`
pour la couverture, celle de `gen_interieur.py` pour l'intérieur — qui décrivaient
les mêmes prestataires sans jamais se recouper.

## Le fichier .ozalid

Une archive, un document :

```
projet.toml     identité du livre, réglages de couverture, chemin source du manuscrit
manuscrit.md
images/         photos source de la 1ère et de la 4ème
```

Le manuscrit y est **copié**, ce qui rend le projet complet sur une autre machine.
Corriger le fichier d'origine ne met donc pas la copie à jour : « Réimporter le
manuscrit » le fait, en un bouton, grâce au chemin mémorisé. L'écart entre les
chapitres attendus et ceux du manuscrit embarqué est affiché — c'est le seul signe
qu'une copie est périmée.

Les **sorties ne sont pas dans l'archive** : elles vont à côté, dans
`<nom-du-projet>/<prestataire>/`. Un projet non enregistré ne peut donc pas
composer, faute d'endroit où écrire.

## Vérifications

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
cd app && node --test "tests/*.test.js"
```

Et les deux exercices sur livre réel, à rejouer après toute modification de la
composition — le compte de pages est ce qu'on compare :

```
cd app/src-tauri
cargo run --example importer -- <livre.toml> <projet.ozalid>
cargo run --example composer -- <projet.ozalid> lulu <sortie>
cargo run --example maquette -- <projet.ozalid> lulu <sortie>
```

`maquette` rend les maquettes en PNG : c'est la vérification qu'aucun test ne peut
faire — la position du cadre, l'assiette du bloc titre, le voile. À rejouer et à
regarder après toute modification du moteur de couverture.

Les tests du front exécutent le vrai `src/app.js` dans un faux DOM qui lit l'état
initial dans le vrai `src/index.html`. Ils couvrent le câblage, jamais le rendu :
tout ce qui se voit se vérifie dans l'application.

## Points d'attention

- **`line-height` CSS ≠ `leading` Typst.** La boîte de ligne est ramenée à 1 em
  (`top-edge: 0.75em, bottom-edge: -0.25em`) pour que les deux grandeurs
  coïncident. Sans cela l'interligne dépend de la police.
- **Le manuscrit n'admet qu'un sous-ensemble de Markdown.** Tout le reste est
  refusé avec son numéro de ligne — un aplatissement silencieux donnerait un
  livre faux, découvert après tirage.
- **Georgia et Helvetica ne sont pas reprises.** Elles appartiennent au système, ne
  sont pas redistribuables, et Helvetica n'existe pas sous Windows. Une maquette
  importée qui les utilise est refusée avec la liste des familles embarquées.
- **Le prolongement panoramique dépend de la pagination.** La 4ème y montre la part
  de l'image située au-delà du dos : le composer sans compte de pages est refusé,
  pas approximé.
- **L'aperçu et le PDF sortent de la même source.** Il n'y a donc pas d'écart
  écran/export à surveiller — le piège que consignait le `CLAUDE.md` du projet
  n'existe plus ici.
- **Le panneau de réglages est construit depuis un schéma** (`src/couverture.js`),
  pas écrit à la main : un chemin faux y laisse un contrôle vide, ce qui se voit
  tout de suite, et un test vérifie que tous les chemins existent.
- **L'icône est provisoire.** Un placeholder, pas une identité visuelle.
