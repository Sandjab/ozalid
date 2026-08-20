# Ozalid Studio

Application de bureau macOS + Windows qui tient la chaîne entière : manuscrit →
intérieur composé → couverture → packages prestataires. Elle succède à la paire
`index.html` + `outils/`, désormais gelée (voir
`docs/superpowers/specs/2026-08-20-ozalid-studio-design.md`).

Ce qu'elle règle : le nombre de pages ne transite plus par un humain. L'intérieur
le produit, la couverture le consomme, et le dos suit le manuscrit sans ressaisie.

**État : jalon 5** — projet `.ozalid`, import d'un livre existant, composition de
l'intérieur, moteur de couverture, assemblage de la planche, packages
multi-prestataires, épreuve de relecture et vérification Windows par intégration
continue : chaque push et chaque pull request compilent, testent et paginent le
témoin sur `windows-latest`, et un tag `v*` produit l'installeur, l'installe en
silencieux pour vérifier son arborescence, et le dépose en release draft. Reste la
vérification manuelle du premier lancement sur une machine Windows — aucun runner
ne lance l'application avec sa fenêtre.

## Stack

- **Tauri 2 + Rust** pour le client, front vanilla sans bundler ni framework.
- **Typst** en sidecar : un binaire statique, sans dépendance système, la même
  version sur les deux plateformes. C'est ce qui rend la pagination reproductible
  d'une machine à l'autre — ni Python, ni pandoc, ni WeasyPrint.

## Mise en route

```
app/outils/typst.sh --local     # ou sans --local pour télécharger la version épinglée
app/outils/polices.sh           # ~10 Mo de polices OFL
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

## Installer sous Windows

L'installeur (`.exe` NSIS) vient de la publication déclenchée par un tag `v*` : le job
`publier` le construit, l'installe en silencieux pour vérifier qu'il pose bien ses
fichiers, puis le dépose en **release draft** sur GitHub — c'est un humain qui publie,
après avoir lancé l'application au moins une fois.

Au premier lancement, Windows affiche « Windows a protégé votre PC » : SmartScreen ne
reconnaît pas l'éditeur tant que le binaire n'est pas signé. Il faut choisir
« Informations complémentaires », puis « Exécuter quand même ». L'installation elle-même
ne demande aucun droit administrateur : elle se fait par utilisateur, dans
`%LOCALAPPDATA%\Ozalid Studio`, où l'application trouve `typst.exe` à côté d'elle et ses
polices dans `fonts\`. Un certificat de signature de code lèverait l'avertissement ; il
n'a pas été pris tant que la diffusion reste confidentielle.

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
| `interieur` | Source Typst de l'intérieur, police du livre, et convergence gouttière/parité |
| `epreuve` | Source Typst de l'épreuve de relecture : A4, numéros de ligne, marge d'annotation |
| `planche` | Assemblage 4ème \| dos \| 1ère au gabarit, et dos composé élément par élément |
| `package` | Un prestataire, un intérieur, une planche, dans son répertoire |
| `commands` | Frontière avec l'interface, et projet ouvert |

`providers` fusionne les deux tables historiques du projet — celle d'`index.html`
pour la couverture, celle de `gen_interieur.py` pour l'intérieur — qui décrivaient
les mêmes prestataires sans jamais se recouper.

## Le fichier .ozalid

Une archive, un document :

```
projet.toml     identité du livre, police de l'intérieur, réglages de couverture,
                chemin source du manuscrit
manuscrit.md
images/         photos source de la 1ère et de la 4ème
```

La police de l'intérieur est une section à part, `[interieur]`, qui vaut `EB Garamond`
quand elle manque — un projet écrit avant qu'elle existe s'ouvre donc sans rien dire.

Le manuscrit y est **copié**, ce qui rend le projet complet sur une autre machine.
Corriger le fichier d'origine ne met donc pas la copie à jour : « Réimporter le
manuscrit » le fait, en un bouton, grâce au chemin mémorisé. L'écart entre les
chapitres attendus et ceux du manuscrit embarqué est affiché — c'est le seul signe
qu'une copie est périmée.

Les **sorties ne sont pas dans l'archive** : elles vont à côté, dans
`<nom-du-projet>/<prestataire>/`. Un projet non enregistré ne peut donc pas
composer, faute d'endroit où écrire. Seule l'épreuve de relecture reste à la racine,
en `epreuve.pdf` : elle ne vise aucun prestataire, elle n'a rien à faire dans leurs
répertoires.

## Vérifications

```
cd app/src-tauri && cargo test --lib && cargo clippy --all-targets && cargo fmt --check
cd app && node --test "tests/*.test.js"
```

Et les exercices sur livre réel, à rejouer après toute modification de la
composition — le compte de pages est ce qu'on compare :

```
cd app/src-tauri
cargo run --example importer -- <livre.toml> <projet.ozalid>
cargo run --example composer -- <projet.ozalid> lulu <sortie>
cargo run --example maquette -- <projet.ozalid> lulu <sortie>
cargo run --example packager -- <projet.ozalid> <sortie> lulu tbe-110x170 bookvault-127x203
cargo run --example epreuve -- <projet.ozalid> <epreuve.pdf>
```

`packager` traverse la chaîne entière sans interface : intérieur composé, pagination
mesurée, dos calculé, planche assemblée. C'est ce qui prouve que Typst compile
vraiment ce que le moteur émet.

`maquette` rend les maquettes en PNG : c'est la vérification qu'aucun test ne peut
faire — la position du cadre, l'assiette du bloc titre, le voile. À rejouer et à
regarder après toute modification du moteur de couverture.

`epreuve` tire l'épreuve de relecture sans interface. Elle se regarde de la même
façon : les numéros de ligne repartent-ils de 1 à chaque page, la marge d'annotation
est-elle libre, un chapitre commence-t-il bien en tête de page.

`temoin` diffère des exercices ci-dessus : lui seul porte sa propre valeur attendue, et
il échoue au lieu d'afficher un résultat à interpréter.

```
cd app/src-tauri && cargo run --example temoin
```

Le manuscrit qu'il compose est *Candide* (Voltaire, 1759, domaine public), versionné
dans `temoin/manuscrit.md` parce que `build/` ne l'est pas et qu'un manuscrit personnel
n'a rien à faire sur un runner GitHub. Sa réussite sous Windows établit que Typst y
pagine comme sur macOS — donc qu'un dos calculé sur l'une des deux plateformes vaut
pour l'autre.

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
- **La police de l'intérieur est un réglage du projet, et elle repagine.** Sept serifs
  de labeur sont admis, EB Garamond par défaut ; le compte de pages, donc le dos, en
  dépend. Une police hors liste est refusée au lieu d'être substituée : Typst, lui,
  composerait dans sa police par défaut sans lever la moindre erreur, et le livre
  sortirait faux en silence.
- **Georgia et Helvetica ne sont pas reprises.** Elles appartiennent au système, ne
  sont pas redistribuables, et Helvetica n'existe pas sous Windows. Une maquette
  importée qui les utilise est refusée avec la liste des familles embarquées.
- **Le prolongement panoramique dépend de la pagination.** L'image y est cadrée sur
  la planche entière — deux couvertures et le dos — et non sur la seule 1ère :
  le composer sans compte de pages est refusé, pas approximé. C'est un écart
  délibéré avec `index.html`, qui cadrait sur une couverture et laissait la 4ème en
  papier nu tant qu'on n'avait pas grossi l'image à la main.
- **La planche ne porte aucun trait de coupe.** Lulu, KDP et Bookvault les refusent
  explicitement ; le fond perdu suffit à dire où couper.
- **Le dos se règle élément par élément.** Auteur, titre et éditeur y ont chacun leur
  style, leur place — pied, centre ou tête — et leur rang, parce que les collections
  ne s'accordent pas là-dessus. Seule sa **largeur** échappe au réglage : elle vient
  de la pagination, et c'est tout l'objet de l'application.
- **L'aperçu et le PDF sortent de la même source.** Il n'y a donc pas d'écart
  écran/export à surveiller — le piège que consignait le `CLAUDE.md` du projet
  n'existe plus ici.
- **Le panneau de réglages est construit depuis un schéma** (`src/couverture.js`),
  pas écrit à la main : un chemin faux y laisse un contrôle vide, ce qui se voit
  tout de suite, et un test vérifie que tous les chemins existent.
- **L'icône est provisoire.** Un placeholder, pas une identité visuelle.
