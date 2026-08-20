# Release Windows par la CI

Date : 2026-08-20
Statut : validé (brainstorming)

## Objectif

Jalon 5, second volet. La spec Ozalid Studio annonce une application « macOS +
Windows » et pose que « Windows n'est testable que via les releases CI »
(`2026-08-20-ozalid-studio-design.md:29`). Aujourd'hui rien ne construit
l'application : le seul workflow du dépôt publie `index.html` sur GitHub Pages,
c'est-à-dire l'atelier gelé.

Ce volet établit deux choses, dans cet ordre :

1. **Que Windows compose à l'identique.** C'est le seul risque technique du
   jalon. Toute la stratégie du projet — Typst en sidecar, version épinglée,
   polices embarquées, `--ignore-system-fonts` — n'a de valeur que si elle tient
   la promesse de reproductibilité d'une plateforme à l'autre. Rien ne l'a
   jamais vérifié.
2. **Qu'un installeur existe**, produit sur tag et attaché à une release.

## Décisions de cadrage (brainstorming du 20/08)

- **Windows seul.** macOS se construit à la main sur le poste de développement ;
  la CI ne prend en charge que ce qui n'est pas faisable localement.
- **Un manuscrit-témoin versionné**, fabriqué pour l'occasion. C'est le seul
  endroit du projet où l'on s'écarte du principe « matériel de test réel » :
  `build/` et `images/` sont ignorés par git, la CI n'a donc aucun livre à
  composer, et faire transiter un manuscrit personnel par un runner GitHub n'est
  pas souhaitable.
- **Aucune image dans le témoin.** La maquette Blanche est purement
  typographique (`maquettes.rs:139`) : le témoin traverse la chaîne entière —
  intérieur, dos, planche — sans qu'un octet binaire entre dans le dépôt.
- **Non signé, assumé et documenté.** Pas de certificat de signature de code.
  SmartScreen avertira au premier lancement ; la marche à suivre est écrite.
- **NSIS seul** (`.exe`), installation par utilisateur, sans droits
  administrateur.
- **Publication sur tag `v*`**, en release **draft** : la CI prépare, l'humain
  publie.
- **Les scripts bash existants sont réutilisés**, pas doublés en PowerShell.

## 1. Le matériel : un manuscrit-témoin

### Ce qui a été constaté

Le témoin de non-régression du projet est le compte de pages des *Heures
creuses*, un manuscrit qui ne peut pas être versionné. La CI ne dispose donc
d'aucun texte à composer, et sans texte elle ne peut que compiler — ce qui ne
prouve rien sur la pagination.

Aucun `.gitattributes` n'existe. Un checkout Windows peut convertir les fins de
ligne en CRLF selon la configuration du runner. Si cela déplaçait la pagination,
on ne saurait pas distinguer un défaut de Typst d'un artefact de git.

### Ce qui est décidé

`app/src-tauri/temoin/manuscrit.md` : un texte écrit pour l'occasion, quatre
chapitres, dimensionné pour composer une vingtaine de pages au gabarit Lulu.

Il n'est pas là pour se lire. Il est là pour porter ce qui casse en traversant
une plateforme :

- accents et caractères composés, sur lesquels se voit une substitution de
  police ;
- apostrophes typographiques `’`, guillemets `« »`, tirets cadratins `—` ;
- italiques, un titre de chapitre assez long pour se replier ;
- une rupture de scène `---`.

La rupture de scène est **présente sans être exigée au rendu**.
`interieur::source` les ignore encore — dette consignée dans `NOTES.md` § 4, et
le test `l_interieur_compose_a_l_identique_avec_ou_sans_rupture_de_scene` fige
cet état. Le témoin ne doit pas le contredire ; il documente au contraire ce qui
tombera le jour où la dette sera traitée.

Un `.gitattributes` à la racine force `*.md text eol=lf`, pour que le manuscrit
soit octet pour octet le même sur les deux plateformes.

## 2. Le juge : `examples/temoin.rs`

### Frontière

Sur le modèle des cinq exemples existants, et pour la même raison : traverser la
chaîne sans interface est la seule façon de prouver que Typst compile vraiment
ce que le moteur émet.

Il construit un `Projet` par `Projet::nouveau` (`projet.rs:110`) à partir du
manuscrit-témoin et de métadonnées écrites dans l'exemple — titre, auteur,
éditeur, maquette Blanche, police par défaut. Il compose ensuite le package Lulu
par `package::assembler`, puis **compare le compte de pages obtenu à une
constante**.

Écart → message affichant les deux valeurs, code de sortie non nul.

### La constante

Elle est relevée sur macOS pendant l'implémentation, puis figée dans l'exemple,
commentée à côté de ce dont elle dépend : Typst 0.15.1, EB Garamond, gabarit
Lulu, corps et interligne du prestataire.

La faire bouger est un acte délibéré, à revalider — jamais un ajustement pour
faire passer la CI. C'est la même discipline que la version épinglée de Typst.

### Le binaire Typst

`packager` instancie `Typst::new("typst")` : le binaire du PATH, pas le sidecar.
`temoin` fait de même, pour rester identique à ce qui tourne en local. La CI
recopie le sidecar sous le nom `typst.exe` dans un répertoire ajouté au PATH du
job : aucune ligne de Rust ne change pour la CI, et l'exemple se lance sur le
poste de développement par `cargo run --example temoin`.

## 3. Le workflow : `.github/workflows/windows.yml`

Un seul fichier, deux jobs. Les deux partagent les étapes d'approvisionnement ;
les dupliquer dans deux workflows ferait de la version épinglée de Typst une
valeur à tenir à deux endroits — la dérive que `providers` a précisément corrigée
en fusionnant les deux tables historiques du projet.

### Job `verifier`

Sur `windows-latest`, à chaque push sur `main` et sur chaque pull request.

- `app/outils/typst.sh x86_64-pc-windows-msvc` et `app/outils/polices.sh`, en
  `shell: bash` — Git Bash est présent sur les runners Windows, et `typst.sh`
  accepte déjà un triple en argument avec sa branche `*windows*`.
- `binaries/` et `fonts/` mis en cache. La clé porte la version épinglée de
  Typst et l'empreinte de `polices.sh` : relever l'une ou modifier l'autre
  invalide le cache, ce qui est le comportement voulu.
- Le sidecar recopié en `typst.exe` dans un répertoire ajouté à `$GITHUB_PATH`.
- `cargo fmt --check`, `cargo clippy --all-targets`, `cargo test --lib`.
- `node --test "tests/*.test.js"`.
- `cargo run --example temoin`.

`typst.sh` a besoin d'un correctif : il appelle `unzip`, dont la présence dans
Git Bash n'est pas garantie. `tar -xf` sait ouvrir un zip sur Windows 10 et
au-delà ; la branche `*windows*` essaie `unzip` puis se rabat sur `tar`.

### Job `publier`

`needs: verifier`, déclenché uniquement sur tag `v*`.

- **Garde-fou de version** : si le tag et le champ `version` de
  `tauri.conf.json` divergent, le job échoue avant de construire. Une release
  dont le numéro ment sur son contenu est pire que pas de release.
- Approvisionnement identique, cache partagé.
- CLI Tauri installée par `npm install -g @tauri-apps/cli` — un binaire
  préconstruit, quelques secondes, là où `cargo install tauri-cli` coûte
  plusieurs minutes de compilation.
- `tauri build --bundles nsis`. Le format est choisi sur la ligne de commande et
  non dans `tauri.conf.json`, qui reste à `targets: "all"` pour que la
  construction macOS locale continue de produire son `.dmg`.
- `gh release create --draft`, le `.exe` attaché, et dans le corps la marche à
  suivre SmartScreen. `gh` est présent sur les runners : pas de dépendance à une
  action tierce.

## 4. Ce que la construction prouve en plus

Après le bundle, le job installe le `.exe` en silencieux (`/S`) et **inspecte
l'arborescence installée** : présence de `typst.exe` à côté de l'exécutable, et
des `fonts/*.ttf` à l'endroit où `commands.rs::typst()` les cherche.

Cela répond au commentaire laissé ouvert dans le code — « Empaquetage macOS :
[…] le chemin réel en release se vérifie au jalon 5 » (`commands.rs:583`) — pour
la plateforme Windows, et sans lancer de fenêtre. Un écart ici signifie une
application qui s'ouvre puis refuse de composer, la panne la plus probable au
premier lancement.

## 5. Ce que la CI ne prouve pas

Aucun runner ne lancera l'application avec sa WebView2 et son interface.

La CI établit que Windows **compile, teste, pagine à l'identique** et que
l'installeur pose ses fichiers aux bons endroits. Que la fenêtre s'ouvre, que le
dialogue de fichier réponde, que l'aperçu s'affiche : cela reste une
**vérification manuelle sur une machine ou une VM Windows**, à faire une fois
avant de publier la première release draft.

C'est une condition de sortie du jalon 5, pas un détail à découvrir plus tard.

## 6. Vérification

- Le workflow au vert sur un push de `main`.
- Le compte de pages du témoin relevé sous Windows **égal** à celui relevé sur
  macOS. C'est l'assertion centrale du volet : si les deux diffèrent, le
  chantier ne se termine pas par un installeur mais par une enquête.
- Un tag d'essai (`v0.1.0`) produit une release draft portant un `.exe`.
- L'inspection de l'arborescence installée au vert.
- Le `.exe` téléchargé, installé et lancé à la main sur une machine Windows :
  la fenêtre s'ouvre, un projet s'importe, un intérieur se compose.

## 7. Documentation

- `app/README.md` : l'état du jalon 5 remplacé — « Reste la release Windows »
  n'est plus vrai —, une section « Installer sous Windows » avec l'avertissement
  SmartScreen et sa marche à suivre, et la mention du témoin dans les
  vérifications.
- Le témoin ajouté à la liste des exercices sur livre réel, avec ce qui le
  distingue : il est le seul à porter sa propre valeur attendue.

## Hors périmètre

- macOS en intégration continue.
- Signature de code, sur l'une ou l'autre plateforme.
- Linux.
- Mise à jour automatique de l'application.
- La correction des ruptures de scène de l'intérieur (dette `NOTES.md` § 4).
- Le portage des scripts d'approvisionnement en PowerShell.
