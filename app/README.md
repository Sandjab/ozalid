# Ozalid Studio

Application de bureau macOS + Windows qui tient la chaîne entière : manuscrit →
intérieur composé → couverture → packages prestataires. Elle succède à la paire
`index.html` + `outils/`, désormais gelée (voir
`docs/superpowers/specs/2026-08-20-ozalid-studio-design.md`).

Ce qu'elle règle : le nombre de pages ne transite plus par un humain. L'intérieur
le produit, la couverture le consomme, et le dos suit le manuscrit sans ressaisie.

**État : jalon 1** — composition de l'intérieur et pagination. Pas encore de
couverture, de `.ozalid` ni de packages.

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
| `projet` | Identité du livre (le `.ozalid` viendra au jalon 2) |
| `typst` | Invocation du sidecar : mesurer la pagination, compiler, rendre un aperçu |
| `interieur` | Source Typst de l'intérieur, et convergence gouttière/parité |
| `commands` | Frontière avec l'interface |

`providers` fusionne les deux tables historiques du projet — celle d'`index.html`
pour la couverture, celle de `gen_interieur.py` pour l'intérieur — qui décrivaient
les mêmes prestataires sans jamais se recouper.

## Vérifications

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
cd app && node --test "tests/*.test.js"
```

Et le témoin de non-régression, à rejouer après toute modification de la
composition — le compte de pages est ce qu'on compare :

```
cd app/src-tauri
cargo run --example composer -- <manuscrit.md> lulu <sortie>
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
- **L'icône est provisoire.** Un placeholder, pas une identité visuelle.
