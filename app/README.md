# Ozalid Studio

Application de bureau macOS + Windows qui tient la chaîne entière : manuscrit →
intérieur composé → couverture → packages prestataires. Elle succède à la paire
`index.html` + `outils/`, désormais gelée (voir
`docs/superpowers/specs/2026-08-20-ozalid-studio-design.md`).

Ce qu'elle règle : le nombre de pages ne transite plus par un humain. L'intérieur
le produit, la couverture le consomme, et le dos suit le manuscrit sans ressaisie.

**État : jalon 2** — projet `.ozalid`, import d'un livre existant, composition de
l'intérieur et pagination. Pas encore de couverture ni de packages.

## Stack

- **Tauri 2 + Rust** pour le client, front vanilla sans bundler ni framework.
- **Typst** en sidecar : un binaire statique, sans dépendance système, la même
  version sur les deux plateformes. C'est ce qui rend la pagination reproductible
  d'une machine à l'autre — ni Python, ni pandoc, ni WeasyPrint.

## Mise en route

```
app/outils/typst.sh --local     # ou sans --local pour télécharger la version épinglée
cd app/src-tauri && cargo tauri dev
```

`typst.sh` place le sidecar dans `src-tauri/binaries/`, répertoire non versionné.
La version de Typst y est **épinglée** : deux versions ne composent pas forcément
le même nombre de pages, donc pas le même dos. La relever est un changement
délibéré, à revalider sur un manuscrit réel.

## Modules

L'interface n'a aucune logique métier : elle invoque des commandes et affiche des
résultats. Tout le reste est testable sans fenêtre.

| Module | Rôle |
|---|---|
| `providers` | Table **unique** des gabarits : format, marges, gouttières, fond perdu, formule de dos |
| `manuscrit` | Markdown → chapitres → contenu Typst, avec refus explicite du non composable |
| `projet` | Le `.ozalid` : lecture, écriture, identité du livre |
| `png` | Lecture du bloc de réglages qu'`index.html` écrit dans ses PNG |
| `import` | Un `livre.toml` de l'ancienne chaîne → un projet |
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
```

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
- **Les réglages de couverture importés ne sont pas encore lus.** Ils sont
  conservés tels quels sous `[couverture.atelier]`, avec leurs identifiants
  d'origine (`inTitre`, `inFrameM`…) et leurs types. Le moteur Typst du jalon 3 les
  traduira ; les figer dans un schéma maison avant que ce moteur existe reviendrait
  à inventer une cible.
- **L'icône est provisoire.** Un placeholder, pas une identité visuelle.
